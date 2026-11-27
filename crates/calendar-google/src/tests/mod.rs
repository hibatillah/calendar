//! Behaviour tests against a local fake HTTP endpoint. Google is never contacted.

use crate::{
    Calendar, Config, Endpoints, Error, EventChange, EventQuery, Google,
    auth::sign_in_scope,
    sign_in::{challenge, query_value},
};
use calendar_core::CalendarEvent;
use chrono::{TimeZone, Utc};
use chrono_tz::Asia::Jakarta;
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

pub(super) struct Request {
    method: String,
    target: String,
    headers: Vec<(String, String)>,
    body: String,
}

impl Request {
    fn path(&self) -> &str {
        self.target.split('?').next().unwrap()
    }

    fn query(&self, key: &str) -> Option<String> {
        query_value(self.target.split_once('?')?.1, key)
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

struct Fake {
    origin: String,
    address: SocketAddr,
    requests: Arc<Mutex<Vec<Request>>>,
    stop: Arc<AtomicBool>,
}

impl Fake {
    fn requests_to(&self, path: &str) -> usize {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|request| request.path() == path)
            .count()
    }

    fn last(&self) -> Request {
        let requests = self.requests.lock().unwrap();
        let last = requests.last().unwrap();

        Request {
            method: last.method.clone(),
            target: last.target.clone(),
            headers: last.headers.clone(),
            body: last.body.clone(),
        }
    }
}

impl Drop for Fake {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(self.address);
    }
}

/// Answers one request per connection; the handler returns a status and a JSON body.
fn serve(handler: impl Fn(&Request) -> (u16, Value) + Send + 'static) -> Fake {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let stop = Arc::new(AtomicBool::new(false));
    let (log, halt) = (requests.clone(), stop.clone());

    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            if halt.load(Ordering::Relaxed) {
                break;
            }
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let mut parts = line.split_whitespace();
            let (method, target) = (
                parts.next().unwrap().to_owned(),
                parts.next().unwrap().to_owned(),
            );
            let mut headers = Vec::new();
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                let Some((name, value)) = header.trim_end().split_once(": ") else {
                    break;
                };
                headers.push((name.to_lowercase(), value.to_owned()));
            }
            let length = headers
                .iter()
                .find(|(name, _)| name == "content-length")
                .map_or(0, |(_, value)| value.parse().unwrap());
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();

            let request = Request {
                method,
                target,
                headers,
                body: String::from_utf8(body).unwrap(),
            };
            let (status, reply) = handler(&request);
            log.lock().unwrap().push(request);
            let reply = reply.to_string();
            let _ = write!(
                reader.get_mut(),
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                reply.len()
            );
        }
    });

    Fake {
        origin: format!("http://{address}"),
        address,
        requests,
        stop,
    }
}

fn google(fake: &Fake) -> Google {
    let config = Config::resolve(
        (Some("test.apps.googleusercontent.com".into()), None),
        (Some("test-secret".into()), None),
    )
    .unwrap();
    let mut google = Google::new(config);
    google.set_display_zone(Jakarta);
    google.endpoints = Endpoints {
        token: format!("{}/token", fake.origin),
        userinfo: format!("{}/userinfo", fake.origin),
        revoke: format!("{}/revoke", fake.origin),
        api: format!("{}/calendar/v3", fake.origin),
    };

    google
}

fn test_calendar(id: &str, role: &str) -> Calendar {
    Calendar {
        id: id.into(),
        summary: "Test".into(),
        primary: true,
        access_role: role.into(),
        background_color: "#123456".into(),
        time_zone: "Asia/Jakarta".into(),
    }
}

// ---- configuration ----

#[test]
fn configuration_needs_both_values_and_the_runtime_value_wins() {
    let id = "web.apps.googleusercontent.com";
    let resolve = |a, b, c, d| Config::resolve((a, b), (c, d));

    assert!(resolve(None, None, None, None).is_none());
    assert!(resolve(Some(id.into()), None, None, None).is_none());
    assert!(resolve(Some("not-a-client".into()), None, Some("s".into()), None).is_none());
    assert!(resolve(Some("  ".into()), None, Some("s".into()), None).is_none());
    let built_in = resolve(None, Some(id), None, Some("compiled")).unwrap();
    assert_eq!(built_in.client_secret, "compiled");
    let overridden = resolve(
        Some("dev.apps.googleusercontent.com".into()),
        Some(id),
        Some("dev".into()),
        Some("compiled"),
    )
    .unwrap();
    assert_eq!(
        (
            overridden.client_id.as_str(),
            overridden.client_secret.as_str()
        ),
        ("dev.apps.googleusercontent.com", "dev")
    );
}

mod api;
mod sign_in;
