//! Browser consent: PKCE, one-use state, and the loopback listener that receives the redirect.

use crate::{Error, Google, SignedIn, auth::sign_in_scope};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use percent_encoding::{NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};
use sha2::{Digest, Sha256};
use std::{
    io::{BufRead, BufReader, ErrorKind, Read, Write},
    net::{Ipv4Addr, TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

const AUTHORIZATION_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(300);
const POLL_INTERVAL: Duration = Duration::from_millis(100);

const SUCCESS_PAGE: &str =
    "<h1>Google connected</h1><p>Return to Calendar. You can close this tab.</p>";
const FAILURE_PAGE: &str = "<h1>Sign-in did not complete</h1><p>Return to Calendar for details. You can close this tab.</p>";

pub(crate) fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// 244 random bits from the OS generator, URL-safe, inside RFC 7636's 43-128 character range.
fn random_token() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

/// A pending browser consent. Dropping it, finishing it or cancelling it closes the listener,
/// which is what makes the state and verifier single-use.
pub struct SignIn {
    listener: TcpListener,
    state: String,
    pub(crate) verifier: String,
    pub(crate) redirect_uri: String,
    authorization_url: String,
    pub(crate) deadline: Instant,
    cancelled: Arc<AtomicBool>,
}

#[derive(Clone)]
pub struct CancelHandle(Arc<AtomicBool>);

impl CancelHandle {
    /// Ends the waiting `complete_sign_in`; nothing is stored, so existing accounts stay untouched.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

impl SignIn {
    /// Open this in the system browser (see `open_in_browser`).
    pub fn authorization_url(&self) -> &str {
        &self.authorization_url
    }

    pub fn cancel_handle(&self) -> CancelHandle {
        CancelHandle(self.cancelled.clone())
    }

    /// Waits for the browser redirect. Unrelated and wrong-state requests are answered and ignored.
    fn wait_for_callback(&self) -> Result<(String, TcpStream), Error> {
        loop {
            if self.cancelled.load(Ordering::Relaxed) {
                return Err(Error::SignInCancelled);
            }
            if Instant::now() >= self.deadline {
                return Err(Error::SignInTimedOut);
            }

            match self.listener.accept() {
                Ok((stream, _)) => {
                    if let Some(outcome) = self.handle_connection(stream) {
                        return outcome;
                    }
                }
                Err(error) if error.kind() == ErrorKind::WouldBlock => {
                    std::thread::sleep(POLL_INTERVAL);
                }
                Err(_) => return Err(Error::ListenerUnavailable),
            }
        }
    }

    fn handle_connection(&self, stream: TcpStream) -> Option<Result<(String, TcpStream), Error>> {
        stream.set_nonblocking(false).ok()?;
        stream.set_read_timeout(Some(Duration::from_secs(2))).ok()?;

        let mut request_line = String::new();
        BufReader::new((&stream).take(8192))
            .read_line(&mut request_line)
            .ok()?;

        match parse_callback(&request_line, &self.state) {
            Callback::Unrelated => respond(stream, "404 Not Found", FAILURE_PAGE),
            Callback::WrongState => respond(stream, "400 Bad Request", FAILURE_PAGE),
            Callback::Denied => {
                respond(stream, "200 OK", FAILURE_PAGE);
                return Some(Err(Error::SignInDenied));
            }
            Callback::Code(code) => return Some(Ok((code, stream))),
        }

        None
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Callback {
    /// Not the redirect (favicon, probes).
    Unrelated,
    WrongState,
    /// The user denied consent, or Google returned no code.
    Denied,
    Code(String),
}

pub(crate) fn query_value(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        (name == key).then(|| percent_decode_str(value).decode_utf8_lossy().into_owned())
    })
}

fn parse_callback(request_line: &str, expected_state: &str) -> Callback {
    let mut parts = request_line.split_whitespace();
    let (Some("GET"), Some(target)) = (parts.next(), parts.next()) else {
        return Callback::Unrelated;
    };
    let Some(("/", query)) = target.split_once('?') else {
        return Callback::Unrelated;
    };
    if query_value(query, "state").as_deref() != Some(expected_state) {
        return Callback::WrongState;
    }
    if query_value(query, "error").is_some() {
        return Callback::Denied;
    }

    query_value(query, "code")
        .filter(|code| !code.is_empty())
        .map_or(Callback::Denied, Callback::Code)
}

fn respond(mut stream: TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'none'\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

/// Opens the consent URL in the default browser. The URL contains the one-use state; never log it.
#[cfg(windows)]
pub fn open_in_browser(url: &str) -> Result<(), Error> {
    // rundll32 receives the URL as one argument, so `&` in the query is not interpreted by a shell.
    std::process::Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", url])
        .spawn()
        .map(drop)
        .map_err(|_| Error::ListenerUnavailable)
}

#[cfg(not(windows))]
pub fn open_in_browser(_url: &str) -> Result<(), Error> {
    Err(Error::ListenerUnavailable)
}

impl Google {
    /// Binds a random loopback port and builds the consent URL. Nothing is stored yet.
    pub fn begin_sign_in(&self) -> Result<SignIn, Error> {
        let listener =
            TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).map_err(|_| Error::ListenerUnavailable)?;
        listener
            .set_nonblocking(true)
            .map_err(|_| Error::ListenerUnavailable)?;
        let port = listener
            .local_addr()
            .map_err(|_| Error::ListenerUnavailable)?
            .port();

        let redirect_uri = format!("http://127.0.0.1:{port}/");
        let state = random_token();
        let verifier = random_token();
        let scope = sign_in_scope();
        let code_challenge = challenge(&verifier);
        let query = [
            ("client_id", self.config.client_id.as_str()),
            ("redirect_uri", &redirect_uri),
            ("response_type", "code"),
            ("scope", &scope),
            ("state", &state),
            ("code_challenge", &code_challenge),
            ("code_challenge_method", "S256"),
            ("access_type", "offline"),
            ("prompt", "consent select_account"),
        ]
        .map(|(name, value)| format!("{name}={}", utf8_percent_encode(value, NON_ALPHANUMERIC)))
        .join("&");

        tracing::info!(action = "google_sign_in_started", port);

        Ok(SignIn {
            listener,
            state,
            verifier,
            redirect_uri,
            authorization_url: format!("{AUTHORIZATION_URL}?{query}"),
            deadline: Instant::now() + SIGN_IN_TIMEOUT,
            cancelled: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Blocks until consent finishes, is denied, is cancelled or five minutes pass, then exchanges the code.
    /// `existing_account_ids` are the accounts already stored; a repeat sign-in is flagged, not duplicated.
    pub fn complete_sign_in(
        &self,
        sign_in: SignIn,
        existing_account_ids: &[String],
    ) -> Result<SignedIn, Error> {
        let (code, stream) = sign_in.wait_for_callback().inspect_err(
            |error| tracing::info!(action = "google_sign_in_ended", outcome = ?error),
        )?;
        let result = self.exchange_code(&sign_in, &code, existing_account_ids);

        respond(
            stream,
            "200 OK",
            if result.is_ok() {
                SUCCESS_PAGE
            } else {
                FAILURE_PAGE
            },
        );
        tracing::info!(action = "google_sign_in_ended", succeeded = result.is_ok());

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_matches_rfc7636_vector() {
        assert_eq!(
            challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn random_tokens_are_long_and_unique() {
        let (first, second) = (random_token(), random_token());

        assert_eq!(first.len(), 64);
        assert_ne!(first, second);
    }

    #[test]
    fn callback_requires_the_exact_state_before_anything_else() {
        let line = |query: &str| format!("GET /?{query} HTTP/1.1\r\n");

        assert_eq!(
            parse_callback(&line("state=good&code=abc%2Fdef"), "good"),
            Callback::Code("abc/def".into())
        );
        assert_eq!(
            parse_callback(&line("state=bad&code=abc"), "good"),
            Callback::WrongState
        );
        assert_eq!(
            parse_callback(&line("code=abc"), "good"),
            Callback::WrongState
        );
        assert_eq!(
            parse_callback(&line("state=bad&error=access_denied"), "good"),
            Callback::WrongState
        );
        assert_eq!(
            parse_callback(&line("state=good&error=access_denied"), "good"),
            Callback::Denied
        );
        assert_eq!(
            parse_callback(&line("state=good"), "good"),
            Callback::Denied
        );
        assert_eq!(
            parse_callback("GET /favicon.ico HTTP/1.1\r\n", "good"),
            Callback::Unrelated
        );
        assert_eq!(
            parse_callback("POST /?state=good&code=a HTTP/1.1\r\n", "good"),
            Callback::Unrelated
        );
    }
}
