use opentelemetry::{
    logs::{LogRecord as _, Logger as _, LoggerProvider as _, Severity},
    propagation::{Extractor, Injector, TextMapPropagator},
    trace::{TraceContextExt, TracerProvider as _},
};
use opentelemetry_otlp::{Protocol, WithExportConfig, WithHttpConfig};
use opentelemetry_sdk::{
    Resource,
    logs::{SdkLogger, SdkLoggerProvider},
    propagation::TraceContextPropagator,
    trace::SdkTracerProvider,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    sync::{Mutex, OnceLock},
    time::Duration,
};
use tracing::{
    Subscriber,
    field::{Field, Visit},
};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::{
    EnvFilter, Layer,
    layer::{Context, SubscriberExt},
    registry::LookupSpan,
    util::SubscriberInitExt,
};

const CAPACITY: usize = 500;
static RECORDS: Mutex<VecDeque<LogEntry>> = Mutex::new(VecDeque::new());
static REMOTE: OnceLock<Remote> = OnceLock::new();
static STATUS: OnceLock<String> = OnceLock::new();

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub component: String,
    pub level: String,
    pub action: String,
    pub details: BTreeMap<String, String>,
}

pub fn recent_logs() -> Vec<LogEntry> {
    RECORDS
        .lock()
        .map(|records| records.iter().cloned().collect())
        .unwrap_or_default()
}

pub fn status() -> &'static str {
    STATUS
        .get()
        .map(String::as_str)
        .unwrap_or("Local only · PostHog is off")
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Configuration {
    enabled: bool,
    project_token: String,
    host: String,
}
impl Configuration {
    fn load() -> Result<Self, &'static str> {
        let dotenv: HashMap<String, String> = match dotenvy::from_path_iter(".env") {
            Ok(entries) => entries
                .collect::<Result<_, _>>()
                .map_err(|_| "Invalid .env syntax · uploads off")?,
            Err(error) if error.not_found() => HashMap::new(),
            Err(_) => return Err("Cannot read .env · uploads off"),
        };
        let value = |name: &str| {
            std::env::var(name)
                .ok()
                .or_else(|| dotenv.get(name).cloned())
        };
        if let Some(configuration) = Self::from_values(value)? {
            return Ok(configuration);
        }
        let configuration = match std::fs::read_to_string(".local/telemetry.json") {
            Ok(contents) => serde_json::from_str::<Self>(&contents)
                .map_err(|_| "Invalid telemetry.json · uploads off")?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(_) => return Err("Cannot read telemetry.json · uploads off"),
        };
        configuration.validate()?;
        Ok(configuration)
    }
    fn from_values(value: impl Fn(&str) -> Option<String>) -> Result<Option<Self>, &'static str> {
        let enabled = value("POSTHOG_ENABLED");
        let token = value("POSTHOG_PROJECT_TOKEN");
        let host = value("POSTHOG_HOST");
        if enabled.is_none() && token.is_none() && host.is_none() {
            return Ok(None);
        }
        let configuration = Self {
            enabled: match enabled.as_deref().unwrap_or("false") {
                "true" => true,
                "false" => false,
                _ => return Err("POSTHOG_ENABLED must be true or false · uploads off"),
            },
            project_token: token.unwrap_or_default(),
            host: host.unwrap_or_else(|| "https://us.i.posthog.com".into()),
        };
        configuration.validate()?;
        Ok(Some(configuration))
    }
    fn validate(&self) -> Result<(), &'static str> {
        if self.enabled
            && (!self.project_token.starts_with("phc_")
                || self.project_token.len() < 12
                || !matches!(
                    self.host.as_str(),
                    "https://us.i.posthog.com" | "https://eu.i.posthog.com"
                ))
        {
            return Err("Invalid PostHog token or region · uploads off");
        }
        Ok(())
    }
}

struct Remote {
    client: posthog_rs::Client,
    identity: String,
    component: String,
}

pub struct TelemetryGuard {
    _file: tracing_appender::non_blocking::WorkerGuard,
    providers: Option<(SdkLoggerProvider, SdkTracerProvider)>,
}
impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        if let Some((logs, traces)) = self.providers.take() {
            // Exporters own blocking clients; never destroy them on a Tokio runtime thread.
            let (sender, receiver) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                if let Some(remote) = REMOTE.get() {
                    remote.client.shutdown();
                }
                let _ = logs.shutdown();
                let _ = traces.shutdown();
                let _ = sender.send(());
            });
            let _ = receiver.recv_timeout(Duration::from_secs(3));
        }
    }
}

fn remote(
    configuration: Configuration,
    component: String,
    identity: String,
) -> Result<(Remote, SdkLoggerProvider, SdkTracerProvider), String> {
    let headers = HashMap::from([(
        "Authorization".to_string(),
        format!("Bearer {}", configuration.project_token),
    )]);
    let resource = Resource::builder()
        .with_service_name(format!("calendar-{component}"))
        .with_attributes([
            opentelemetry::KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
            opentelemetry::KeyValue::new("deployment.environment.name", "local"),
        ])
        .build();
    let logs = opentelemetry_otlp::LogExporter::builder()
        .with_http()
        .with_endpoint(format!("{}/i/v1/logs", configuration.host))
        .with_protocol(Protocol::HttpBinary)
        .with_headers(headers.clone())
        .with_timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| "Log exporter initialization failed")?;
    let traces = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_endpoint(format!("{}/i/v1/traces", configuration.host))
        .with_protocol(Protocol::HttpBinary)
        .with_headers(headers)
        .with_timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| "Trace exporter initialization failed")?;
    let options = posthog_rs::ClientOptionsBuilder::default()
        .api_key(configuration.project_token)
        .host(configuration.host)
        .disable_geoip(true)
        .is_server(component == "backend")
        .request_timeout_seconds(10)
        .shutdown_timeout_ms(1500)
        .max_queue_size(500)
        .build()
        .map_err(|_| "Analytics initialization failed")?;
    Ok((
        Remote {
            client: posthog_rs::client(options),
            identity,
            component,
        },
        SdkLoggerProvider::builder()
            .with_resource(resource.clone())
            .with_batch_exporter(logs)
            .build(),
        SdkTracerProvider::builder()
            .with_resource(resource)
            .with_batch_exporter(traces)
            .build(),
    ))
}

pub fn init(component: &str) -> TelemetryGuard {
    let directory = std::env::var("CALENDAR_LOG_DIR").unwrap_or_else(|_| ".local/logs".into());
    let appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix(component)
        .filename_suffix("jsonl")
        .max_log_files(7)
        .build(directory)
        .expect("log directory must be writable");
    let (writer, file_guard) = tracing_appender::non_blocking(appender);
    let providers = match Configuration::load() {
        Ok(configuration) if configuration.enabled => {
            let component = component.to_owned();
            match std::thread::spawn(move || {
                let identity_path = format!(".local/{component}-telemetry-id");
                let identity = std::fs::read_to_string(&identity_path)
                    .ok()
                    .filter(|value| uuid::Uuid::parse_str(value).is_ok())
                    .unwrap_or_else(|| {
                        let identity = uuid::Uuid::new_v4().to_string();
                        let _ = std::fs::write(identity_path, &identity);
                        identity
                    });
                remote(configuration, component, identity)
            })
            .join()
            {
                Ok(Ok((remote, logs, traces))) => {
                    let _ = REMOTE.set(remote);
                    let providers = (logs, traces);
                    let _ = STATUS.set("PostHog enabled · delivery is best effort".into());
                    Some(providers)
                }
                _ => {
                    let _ =
                        STATUS.set("PostHog initialization failed · local logs available".into());
                    None
                }
            }
        }
        Ok(_) => {
            let _ = STATUS.set("Local only · PostHog is off".into());
            None
        }
        Err(message) => {
            let _ = STATUS.set(message.into());
            None
        }
    };
    let logger = providers.as_ref().map(|(logs, _)| logs.logger("calendar"));
    let trace_layer = providers.as_ref().map(|(_, traces)| {
        tracing_opentelemetry::layer()
            .with_tracer(traces.tracer("calendar"))
            .with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
                metadata.is_span() && metadata.target().starts_with("calendar")
            }))
    });
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,wgpu=error,naga=error".into()),
        )
        .with(tracing_subscriber::fmt::layer().json().with_writer(writer))
        .with(trace_layer)
        .with(ActivityLayer {
            component: component.into(),
            logger,
        })
        .init();
    tracing::info!(action = "application_started");
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        // Panic payloads can contain calendar data. Keep only a stable category remotely.
        report_error("application_panic");
        previous_hook(panic);
    }));
    TelemetryGuard {
        _file: file_guard,
        providers,
    }
}

#[derive(Default)]
struct SafeFields(BTreeMap<String, String>);
impl Visit for SafeFields {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if matches!(
            field.name(),
            "action" | "online" | "revision" | "days" | "visible" | "mode" | "screen" | "control"
        ) {
            self.0.insert(
                field.name().into(),
                format!("{value:?}")
                    .trim_matches('"')
                    .chars()
                    .take(120)
                    .collect(),
            );
        }
    }
}
struct ActivityLayer {
    component: String,
    logger: Option<SdkLogger>,
}
impl<SubscriberType> Layer<SubscriberType> for ActivityLayer
where
    SubscriberType: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn on_event(&self, event: &tracing::Event<'_>, _: Context<'_, SubscriberType>) {
        let metadata = event.metadata();
        // Export only our instrumented actions; exporter diagnostics must not feed themselves.
        if !metadata.target().starts_with("calendar") {
            if matches!(
                *metadata.level(),
                tracing::Level::WARN | tracing::Level::ERROR
            ) && (metadata.target().starts_with("posthog")
                || metadata.target().starts_with("opentelemetry"))
                && let Ok(mut records) = RECORDS.lock()
            {
                push_bounded(
                    &mut records,
                    LogEntry {
                        timestamp: chrono::Utc::now()
                            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                        component: self.component.clone(),
                        level: metadata.level().to_string(),
                        action: "telemetry_transport_warning".into(),
                        details: BTreeMap::new(),
                    },
                );
            }
            return;
        }
        let mut fields = SafeFields::default();
        event.record(&mut fields);
        let action = fields
            .0
            .remove("action")
            .unwrap_or_else(|| metadata.name().into());
        let entry = LogEntry {
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            component: self.component.clone(),
            level: metadata.level().to_string(),
            action: action.clone(),
            details: fields.0,
        };
        if let Ok(mut records) = RECORDS.lock() {
            push_bounded(&mut records, entry.clone());
        }
        if let Some(logger) = &self.logger {
            let mut record = logger.create_log_record();
            record.set_body(action.into());
            record.set_severity_text(metadata.level().as_str());
            record.set_severity_number(match *metadata.level() {
                tracing::Level::ERROR => Severity::Error,
                tracing::Level::WARN => Severity::Warn,
                _ => Severity::Info,
            });
            for (key, value) in entry.details {
                record.add_attribute(key, value);
            }
            let context = tracing::Span::current().context();
            let span = context.span();
            let span_context = span.span_context();
            if span_context.is_valid() {
                record.set_trace_context(
                    span_context.trace_id(),
                    span_context.span_id(),
                    Some(span_context.trace_flags()),
                );
            }
            logger.emit(record);
        }
    }
}
fn push_bounded(records: &mut VecDeque<LogEntry>, entry: LogEntry) {
    if records.len() >= CAPACITY {
        records.pop_front();
    }
    records.push_back(entry);
}

// Call sites pass static categories, never user-entered text or serialized events.
pub fn track(event_name: &'static str, control: &'static str) {
    tracing::info!(action = event_name, control);
    if let Some(remote) = REMOTE.get() {
        let mut event = posthog_rs::Event::new(event_name, &remote.identity);
        let _ = event.insert_prop("control", control);
        let _ = event.insert_prop("component", &remote.component);
        let _ = event.insert_prop("app_version", env!("CARGO_PKG_VERSION"));
        let _ = event.insert_prop("$process_person_profile", false);
        remote.client.capture(event);
    }
}

pub fn report_error(category: &'static str) {
    tracing::error!(action = category);
    if let Some(remote) = REMOTE.get() {
        let error = std::io::Error::other(category);
        let options = posthog_rs::CaptureExceptionOptions::new().fingerprint(category);
        let _ = remote.client.capture_exception_with(&error, options);
    }
}

pub fn outgoing_trace_headers() -> HashMap<String, String> {
    struct Headers(HashMap<String, String>);
    impl Injector for Headers {
        fn set(&mut self, key: &str, value: String) {
            self.0.insert(key.into(), value);
        }
    }
    let mut headers = Headers(HashMap::new());
    TraceContextPropagator::new().inject_context(&tracing::Span::current().context(), &mut headers);
    headers.0
}
pub fn accept_trace_parent(span: &tracing::Span, parent: &str) {
    struct Parent<'a>(&'a str);
    impl Extractor for Parent<'_> {
        fn get(&self, key: &str) -> Option<&str> {
            (key == "traceparent").then_some(self.0)
        }
        fn keys(&self) -> Vec<&str> {
            vec!["traceparent"]
        }
    }
    let context = TraceContextPropagator::new().extract(&Parent(parent));
    let _ = span.set_parent(context);
}

#[cfg(test)]
mod tests {
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
                    if let Some(boundary) = bytes.windows(4).position(|chunk| chunk == b"\r\n\r\n")
                    {
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
}
