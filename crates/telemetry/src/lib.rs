mod configuration;
use configuration::Configuration;
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
mod tests;
