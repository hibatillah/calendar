//! Sign-in over a real loopback listener, token refresh and revoke.

use super::*;

fn token_server() -> Fake {
    serve(|request| match request.path() {
        "/token" => (
            200,
            json!({"access_token": "access-1", "refresh_token": "refresh-1", "expires_in": 3600, "scope": sign_in_scope()}),
        ),
        "/userinfo" => (
            200,
            json!({"sub": "subject-1", "email": "a@example.com", "name": "A", "picture": "https://example.com/p.png"}),
        ),
        _ => (404, json!({})),
    })
}

/// Plays the browser: one GET to the redirect URI. Returns the raw response.
fn browse(redirect_uri: &str, query: &str) -> String {
    let mut stream = TcpStream::connect(
        redirect_uri
            .trim_start_matches("http://")
            .trim_end_matches('/'),
    )
    .unwrap();
    write!(stream, "GET /?{query} HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();

    response
}

fn authorization(url: &str, key: &str) -> String {
    query_value(url.split_once('?').unwrap().1, key).unwrap()
}

#[test]
fn loopback_sign_in_uses_pkce_ignores_wrong_state_and_is_one_use() {
    let fake = token_server();
    let google = google(&fake);
    let sign_in = google.begin_sign_in().unwrap();
    let url = sign_in.authorization_url().to_owned();
    let redirect_uri = authorization(&url, "redirect_uri");
    let state = authorization(&url, "state");

    assert!(url.starts_with("https://accounts.google.com/o/oauth2/v2/auth?"));
    assert_eq!(authorization(&url, "code_challenge_method"), "S256");
    assert_eq!(authorization(&url, "scope"), sign_in_scope());
    assert!(!url.contains("client_secret") && !url.contains("test-secret"));
    assert!(redirect_uri.starts_with("http://127.0.0.1:"));

    let (callback, wrong_state) = (redirect_uri.clone(), "state=wrong&code=stolen".to_owned());
    let browser = std::thread::spawn(move || {
        let rejected = browse(&callback, &wrong_state);
        let accepted = browse(&callback, &format!("state={state}&code=code-1"));
        (rejected, accepted)
    });
    let signed_in = google.complete_sign_in(sign_in, &[]).ok().unwrap();
    let (rejected, accepted) = browser.join().unwrap();

    assert!(rejected.starts_with("HTTP/1.1 400") && rejected.contains("did not complete"));
    assert!(accepted.contains("Google connected") && accepted.contains("no-store"));
    assert_eq!(signed_in.credentials.account.account_id, "subject-1");
    assert_eq!(signed_in.credentials.refresh_token, "refresh-1");
    assert!(!signed_in.already_connected);
    assert!(signed_in.credentials.permissions().write_events);
    assert!(signed_in.access_token.is_fresh());

    // The stolen code never reached the token endpoint, and the verifier matches the challenge.
    assert_eq!(fake.requests_to("/token"), 1);
    let token_request = fake
        .requests
        .lock()
        .unwrap()
        .iter()
        .find(|request| request.path() == "/token")
        .map(|request| request.body.clone())
        .unwrap();
    assert_eq!(
        query_value(&token_request, "code").as_deref(),
        Some("code-1")
    );
    assert_eq!(
        query_value(&token_request, "redirect_uri"),
        Some(redirect_uri.clone())
    );
    assert_eq!(
        challenge(&query_value(&token_request, "code_verifier").unwrap()),
        authorization(&url, "code_challenge")
    );
    assert_eq!(fake.last().header("authorization"), Some("Bearer access-1"));

    // One use: the listener is gone, so the same callback cannot be replayed.
    assert!(
        TcpStream::connect(
            redirect_uri
                .trim_start_matches("http://")
                .trim_end_matches('/')
        )
        .is_err()
    );
}

#[test]
fn denied_consent_ends_the_flow_without_touching_tokens() {
    let fake = token_server();
    let google = google(&fake);
    let sign_in = google.begin_sign_in().unwrap();
    let url = sign_in.authorization_url().to_owned();
    let redirect_uri = authorization(&url, "redirect_uri");
    let state = authorization(&url, "state");

    let callback = redirect_uri.clone();
    let browser = std::thread::spawn(move || {
        browse(&callback, &format!("state={state}&error=access_denied"))
    });
    let outcome = google.complete_sign_in(sign_in, &["subject-1".into()]);

    assert_eq!(outcome.err(), Some(Error::SignInDenied));
    assert!(browser.join().unwrap().contains("did not complete"));
    assert_eq!(fake.requests.lock().unwrap().len(), 0);
    assert!(
        TcpStream::connect(
            redirect_uri
                .trim_start_matches("http://")
                .trim_end_matches('/')
        )
        .is_err()
    );
}

#[test]
fn repeat_sign_in_of_a_known_account_is_flagged() {
    let fake = token_server();
    let google = google(&fake);
    let sign_in = google.begin_sign_in().unwrap();
    let url = sign_in.authorization_url().to_owned();
    let (callback, state) = (
        authorization(&url, "redirect_uri"),
        authorization(&url, "state"),
    );

    let browser =
        std::thread::spawn(move || browse(&callback, &format!("state={state}&code=code-2")));
    let signed_in = google
        .complete_sign_in(sign_in, &["subject-1".into()])
        .ok()
        .unwrap();
    browser.join().unwrap();

    assert!(signed_in.already_connected);
    assert_eq!(signed_in.credentials.account.account_id, "subject-1");
}

#[test]
fn cancel_and_timeout_end_the_wait_without_requests() {
    let fake = token_server();
    let google = google(&fake);

    let sign_in = google.begin_sign_in().unwrap();
    let handle = sign_in.cancel_handle();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        handle.cancel();
    });
    assert_eq!(
        google.complete_sign_in(sign_in, &[]).err(),
        Some(Error::SignInCancelled)
    );

    let mut expired = google.begin_sign_in().unwrap();
    expired.deadline = Instant::now();
    assert_eq!(
        google.complete_sign_in(expired, &[]).err(),
        Some(Error::SignInTimedOut)
    );
    assert_eq!(fake.requests.lock().unwrap().len(), 0);
}

#[test]
fn refresh_revoke_and_rejected_grants() {
    let fake = serve(|request| {
        match (
            request.path(),
            query_value(&request.body, "refresh_token").as_deref(),
        ) {
            ("/token", Some("good")) => (
                200,
                json!({"access_token": "access-2", "expires_in": 3600, "scope": sign_in_scope()}),
            ),
            ("/token", _) => (400, json!({"error": "invalid_grant"})),
            ("/revoke", _) => (200, json!({})),
            _ => (404, json!({})),
        }
    });
    let google = google(&fake);

    let token = google.refresh_access_token("good").ok().unwrap();
    assert_eq!(token.value(), "access-2");
    assert!(token.permissions().read_events);
    assert_eq!(
        google.refresh_access_token("revoked").err(),
        Some(Error::TokenRejected)
    );

    google.revoke("good").unwrap();
    assert_eq!(
        query_value(&fake.last().body, "token").as_deref(),
        Some("good")
    );
}
