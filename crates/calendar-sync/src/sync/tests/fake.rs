//! A tiny local HTTP server that answers with the status and JSON a handler returns.

use serde_json::Value;
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

pub(crate) struct Request {
    pub(crate) method: String,
    pub(crate) path: String,
    query: String,
    pub(crate) authorization: String,
    pub(crate) if_match: String,
    pub(crate) body: String,
}

impl Request {
    pub(crate) fn param(&self, key: &str) -> Option<&str> {
        self.query
            .split('&')
            .chain(self.body.split('&'))
            .find_map(|pair| pair.strip_prefix(key)?.strip_prefix('='))
    }
}

pub(crate) struct Fake {
    pub(crate) origin: String,
    address: SocketAddr,
    pub(crate) requests: Arc<Mutex<Vec<(String, String)>>>,
    calls: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
}

impl Fake {
    /// How many Calendar API requests (not token requests) used `method`.
    pub(crate) fn count(&self, method: &str) -> usize {
        let prefix = format!("{method} /calendar/");

        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|called| called.starts_with(&prefix))
            .count()
    }

    /// Query strings of the requests made to `path`, oldest first.
    pub(crate) fn queries(&self, path: &str) -> Vec<String> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|(requested, _)| requested == path)
            .map(|(_, query)| query.clone())
            .collect()
    }
}

impl Drop for Fake {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(self.address);
    }
}

/// Answers one request per connection with the status and JSON body the handler returns.
pub(crate) fn serve(handler: impl Fn(&Request) -> (u16, Value) + Send + 'static) -> Fake {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let calls = Arc::new(Mutex::new(Vec::new()));
    let stop = Arc::new(AtomicBool::new(false));
    let (log, method_log, halt) = (requests.clone(), calls.clone(), stop.clone());

    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            if halt.load(Ordering::Relaxed) {
                break;
            }
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let method = line.split_whitespace().next().unwrap().to_owned();
            let target = line.split_whitespace().nth(1).unwrap().to_owned();
            let (path, query) = target.split_once('?').unwrap_or((&target, ""));
            let (mut length, mut authorization, mut if_match) = (0, String::new(), String::new());
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                let Some((name, value)) = header.trim_end().split_once(": ") else {
                    break;
                };
                match name.to_lowercase().as_str() {
                    "content-length" => length = value.parse().unwrap(),
                    "authorization" => authorization = value.to_owned(),
                    "if-match" => if_match = value.to_owned(),
                    _ => {}
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();

            let request = Request {
                method,
                path: path.to_owned(),
                query: query.to_owned(),
                authorization,
                if_match,
                body: String::from_utf8(body).unwrap(),
            };
            let (status, reply) = handler(&request);
            method_log
                .lock()
                .unwrap()
                .push(format!("{} {}", request.method, request.path));
            log.lock().unwrap().push((request.path, request.query));
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
        calls,
        stop,
    }
}
