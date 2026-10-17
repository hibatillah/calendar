use super::*;
#[test]
fn environment_configuration_requires_explicit_opt_in() {
    let values = HashMap::from([
        ("POSTHOG_PROJECT_TOKEN", "phc_test_project"),
        ("POSTHOG_HOST", "https://eu.i.posthog.com"),
    ]);
    let configuration =
        Configuration::from_values(|key| values.get(key).map(|value| (*value).into()))
            .unwrap()
            .unwrap();
    assert!(!configuration.enabled);
    assert_eq!(configuration.host, "https://eu.i.posthog.com");
    let configuration = Configuration::from_values(|key| {
        if key == "POSTHOG_ENABLED" {
            Some("true".into())
        } else {
            values.get(key).map(|value| (*value).into())
        }
    })
    .unwrap()
    .unwrap();
    assert!(configuration.enabled);
    assert!(
        Configuration::from_values(|key| (key == "POSTHOG_ENABLED").then(|| "typo".into()))
            .is_err()
    );
    assert!(Configuration::from_values(|_| None).unwrap().is_none());
}

#[test]
fn exporters_send_analytics_errors_logs_and_linked_traces_to_local_receiver() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let host = format!("http://{}", listener.local_addr().unwrap());
    let receiver = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(15);
        let mut requests = Vec::new();
        while requests.len() < 3 && std::time::Instant::now() < deadline {
            let Ok((mut stream, _)) = listener.accept() else {
                std::thread::sleep(Duration::from_millis(10));
                continue;
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0; 4096];
            loop {
                let length = stream.read(&mut buffer).unwrap();
                if length == 0 {
                    break;
                }
                bytes.extend_from_slice(&buffer[..length]);
                if let Some(boundary) = bytes.windows(4).position(|chunk| chunk == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..boundary]).to_lowercase();
                    let content_length = headers
                        .lines()
                        .find_map(|line| {
                            line.strip_prefix("content-length:")
                                .map(|value| value.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if bytes.len() >= boundary + 4 + content_length {
                        break;
                    }
                }
            }
            let is_otlp = bytes.starts_with(b"POST /i/v1/");
            let response = if is_otlp {
                "HTTP/1.1 200 OK\r\ncontent-type: application/x-protobuf\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
            } else {
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 12\r\nconnection: close\r\n\r\n{\"status\":1}"
            };
            stream.write_all(response.as_bytes()).unwrap();
            requests.push(bytes);
        }
        requests
    });
    let (remote, logs, traces) = remote(
        Configuration {
            enabled: true,
            project_token: "phc_test_only".into(),
            host,
        },
        "test".into(),
        "anonymous-test".into(),
    )
    .unwrap();
    let subscriber = tracing_subscriber::registry()
        .with(
            tracing_opentelemetry::layer()
                .with_tracer(traces.tracer("test"))
                .with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
                    metadata.is_span()
                })),
        )
        .with(ActivityLayer {
            component: "test".into(),
            logger: Some(logs.logger("test")),
        });
    tracing::subscriber::with_default(subscriber, || {
        let span = tracing::info_span!("test_request");
        let parent = "00-0123456789abcdef0123456789abcdef-0123456789abcdef-01";
        accept_trace_parent(&span, parent);
        let _entered = span.enter();
        let headers = outgoing_trace_headers();
        assert!(headers["traceparent"].contains("0123456789abcdef0123456789abcdef"));
        tracing::info!(
            action = "test_action",
            title = "NEVER_EXPORT_PRIVATE_TITLE",
            token = "NEVER_EXPORT_SECRET"
        );
        let mut event = posthog_rs::Event::new("test_screen", "anonymous-test");
        event.insert_prop("$process_person_profile", false).unwrap();
        remote.client.capture(event);
        remote
            .client
            .capture_exception(&std::io::Error::other("test_error_category"))
            .unwrap();
    });
    remote.client.flush();
    logs.force_flush().unwrap();
    traces.force_flush().unwrap();
    let requests = receiver.join().unwrap();
    assert_eq!(requests.len(), 3);
    for endpoint in ["/i/v1/logs", "/i/v1/traces"] {
        let request = requests
            .iter()
            .find(|request| {
                String::from_utf8_lossy(request).starts_with(&format!("POST {endpoint}"))
            })
            .unwrap();
        assert!(
            String::from_utf8_lossy(request)
                .to_lowercase()
                .contains("authorization: bearer phc_test_only")
        );
    }
    let payloads = requests
        .iter()
        .map(|request| String::from_utf8_lossy(request))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(payloads.contains("test_screen"));
    assert!(payloads.contains("$exception"));
    assert!(!payloads.contains("NEVER_EXPORT"));
    remote.client.shutdown();
    logs.shutdown().unwrap();
    traces.shutdown().unwrap();
}

#[test]
fn configuration_is_disabled_by_default_and_rejects_secrets_and_arbitrary_hosts() {
    assert!(!Configuration::default().enabled);
    for (token, host) in [
        ("phx_private_secret", "https://us.i.posthog.com"),
        ("phc_project_token", "http://untrusted.example"),
    ] {
        assert!(
            Configuration {
                enabled: true,
                project_token: token.into(),
                host: host.into()
            }
            .validate()
            .is_err()
        );
    }
}

#[test]
fn ring_keeps_only_latest_entries() {
    let mut records = VecDeque::new();
    for index in 0..600 {
        push_bounded(
            &mut records,
            LogEntry {
                timestamp: String::new(),
                component: "test".into(),
                level: "INFO".into(),
                action: index.to_string(),
                details: BTreeMap::new(),
            },
        );
    }
    assert_eq!(records.len(), 500);
    assert_eq!(records.front().unwrap().action, "100");
}

#[test]
fn diagnostic_fields_exclude_calendar_content_and_credentials() {
    struct Capture;
    impl<S: Subscriber> Layer<S> for Capture {
        fn on_event(&self, event: &tracing::Event<'_>, _: Context<'_, S>) {
            let mut fields = SafeFields::default();
            event.record(&mut fields);
            assert_eq!(fields.0.len(), 1);
            assert_eq!(fields.0["action"], "save_requested");
        }
    }
    let subscriber = tracing_subscriber::registry().with(Capture);
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(
            action = "save_requested",
            title = "private meeting",
            token = "secret",
            error = "private URL",
            event_id = "private id"
        )
    });
}
