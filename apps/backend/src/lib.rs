pub mod accounts;
pub mod google;
pub mod repository;
use async_graphql::{
    Context, EmptySubscription, ErrorExtensions, InputObject, Object, Schema, SimpleObject,
};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    Json, Router,
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use calendar_core::{CalendarEvent, EventCommand};
use repository::{Repository, RepositoryError};
use serde::{Deserialize, Serialize};
use tracing::Instrument;

/// A Google calendar event. Editing depends on account permissions and calendar access.
#[derive(Clone, Serialize, Deserialize, SimpleObject, InputObject)]
#[graphql(input_name = "EventInput")]
#[serde(rename_all = "camelCase")]
pub struct EventRecord {
    /// Stable event identifier. Generate a UUID for a new event.
    pub event_id: String,
    /// Provider calendar identifier.
    pub calendar_id: String,
    /// Non-empty title, at most 1000 UTF-8 bytes.
    pub title: String,
    /// Inclusive start instant in RFC 3339 format, including a UTC offset.
    pub start: String,
    /// Exclusive end instant in RFC 3339 format; must be after start.
    pub end: String,
    /// IANA time zone used to display and edit local times, for example Asia/Jakarta.
    pub timezone: String,
    /// All-day events use an exclusive end date.
    pub all_day: bool,
    /// Optional notes, represented as an empty string when absent; maximum 16000 UTF-8 bytes.
    pub description: String,
    /// Location or meeting URL; empty when absent.
    pub location: String,
    /// Non-negative RGB integer, for example 9215736.
    pub color: i32,
    /// Server-managed revision. Use expectedRevision for concurrency checks.
    pub revision: i64,
    /// Whether edits are forbidden for this stored event.
    pub read_only: bool,
}
impl From<CalendarEvent> for EventRecord {
    fn from(event: CalendarEvent) -> Self {
        Self {
            event_id: event.event_id,
            calendar_id: event.calendar_id,
            title: event.title,
            start: event.start.to_rfc3339(),
            end: event.end.to_rfc3339(),
            timezone: event.timezone,
            all_day: event.all_day,
            description: event.description,
            location: event.location,
            color: event.color as i32,
            revision: event.revision,
            read_only: event.read_only,
        }
    }
}
fn repository_error(error: RepositoryError) -> async_graphql::Error {
    let code = match &error {
        RepositoryError::Invalid(_) => "INVALID_EVENT",
        RepositoryError::Conflict(_) => "CONFLICT",
        RepositoryError::ReadOnly => "READ_ONLY",
        RepositoryError::Storage(_) => {
            randevu_telemetry::report_error("graphql_storage_failed");
            "INTERNAL_ERROR"
        }
    };
    async_graphql::Error::new(error.to_string())
        .extend_with(|_, extensions| extensions.set("code", code))
}
fn invalid_input(message: &'static str) -> async_graphql::Error {
    async_graphql::Error::new(message)
        .extend_with(|_, extensions| extensions.set("code", "INVALID_EVENT"))
}
/// Read operations for Google calendars.
pub struct Query;
#[Object]
impl Query {
    /// Google account connection, event permission status, and discovered calendars.
    async fn google_connection(&self, context: &Context<'_>) -> accounts::Connection {
        context
            .data_unchecked::<accounts::GoogleAccounts>()
            .status()
            .await
    }

    /// Google-backed calendar with local offline caching in the desktop.
    async fn mode(&self) -> &str {
        "GOOGLE_CALENDAR"
    }
    /// Load expanded Google occurrences across connected accounts in a bounded date range.
    async fn events(
        &self,
        context: &Context<'_>,
        start_date: Option<String>,
        end_date: Option<String>,
    ) -> async_graphql::Result<Vec<EventRecord>> {
        let start = start_date
            .map(|value| value.parse())
            .transpose()
            .map_err(|_| invalid_input("Invalid start date."))?
            .unwrap_or_else(|| chrono::Utc::now().date_naive() - chrono::Duration::days(14));
        let end = end_date
            .map(|value| value.parse())
            .transpose()
            .map_err(|_| invalid_input("Invalid end date."))?
            .unwrap_or(start + chrono::Duration::days(75));
        Ok(context
            .data_unchecked::<accounts::GoogleAccounts>()
            .events(start, end)
            .await
            .map_err(async_graphql::Error::new)?
            .into_iter()
            .map(Into::into)
            .collect())
    }
}
/// Changes to calendar events. Every write is journaled and deduplicated by operationId.
pub struct Mutation;
#[Object]
impl Mutation {
    /// Start Desktop OAuth with PKCE. Omit accountId to add an account; provide it to renew permissions. Open the returned URL in the system browser.
    async fn connect_google(
        &self,
        context: &Context<'_>,
        account_id: Option<String>,
    ) -> async_graphql::Result<String> {
        context
            .data_unchecked::<accounts::GoogleAccounts>()
            .begin(account_id)
            .await
            .map_err(async_graphql::Error::new)
    }
    /// Remove local Google credentials and attempt to revoke the Google grant. Also cancels pending sign-in.
    async fn disconnect_google(
        &self,
        context: &Context<'_>,
        account_id: Option<String>,
    ) -> async_graphql::Result<bool> {
        context
            .data_unchecked::<accounts::GoogleAccounts>()
            .disconnect(account_id)
            .await
            .map_err(async_graphql::Error::new)
    }

    /// Create or update a Google event. Retries use stable creation IDs and provider operation markers; updates use ETags.
    /// Reusing that ID with different data or saving an outdated revision returns extensions.code CONFLICT.
    /// Validation failures return INVALID_EVENT; stored read-only events return READ_ONLY.
    async fn save_event(
        &self,
        context: &Context<'_>,
        #[graphql(
            desc = "Unique command ID. Reuse only for an identical retry; generate a new UUID for each edit."
        )]
        operation_id: String,
        #[graphql(desc = "Use -1 to create a new event; use its current revision to update it.")]
        expected_revision: i64,
        #[graphql(desc = "Complete event values. The returned revision is authoritative.")]
        event: EventRecord,
    ) -> async_graphql::Result<EventRecord> {
        let command = EventCommand {
            operation_id,
            expected_revision,
            event: CalendarEvent {
                event_id: event.event_id,
                calendar_id: event.calendar_id,
                title: event.title,
                start: event
                    .start
                    .parse()
                    .map_err(|_| invalid_input("Start must be an RFC 3339 timestamp."))?,
                end: event
                    .end
                    .parse()
                    .map_err(|_| invalid_input("End must be an RFC 3339 timestamp."))?,
                timezone: event.timezone,
                all_day: event.all_day,
                description: event.description,
                location: event.location,
                color: u32::try_from(event.color)
                    .map_err(|_| invalid_input("Color must be non-negative."))?,
                revision: event.revision,
                read_only: event.read_only,
            },
        };
        if command.event.calendar_id.contains('|') {
            return context
                .data_unchecked::<accounts::GoogleAccounts>()
                .save(command)
                .await
                .map(Into::into)
                .map_err(async_graphql::Error::new);
        }
        Ok(context
            .data_unchecked::<Repository>()
            .save(command)
            .await
            .map_err(repository_error)?
            .into())
    }
}
pub type CalendarSchema = Schema<Query, Mutation, EmptySubscription>;
pub fn schema(repository: Repository) -> CalendarSchema {
    Schema::build(Query, Mutation, EmptySubscription)
        .data(repository)
        .data(accounts::GoogleAccounts::default())
        .limit_depth(16)
        .limit_complexity(1000)
        .finish()
}
#[derive(Clone)]
struct AppState {
    schema: CalendarSchema,
    token: String,
}
pub fn router(repository: Repository, token: String) -> Router {
    let state = AppState {
        schema: schema(repository),
        token,
    };
    let protected = Router::new()
        .route("/graphql", post(graphql))
        .route("/diagnostics/logs", get(diagnostics))
        .route_layer(middleware::from_fn_with_state(state.clone(), authorize));
    Router::new()
        .merge(protected)
        .route(
            "/health",
            get(|| async {
                Json(serde_json::json!({"status":"healthy","mode":"GOOGLE_CALENDAR"}))
            }),
        )
        .route(
            "/docs",
            get(|| async { Html(include_str!("../graphiql.html")) }),
        )
        .route(
            "/schema.graphql",
            get(|State(state): State<AppState>| async move {
                (
                    [("content-type", "text/plain; charset=utf-8")],
                    state.schema.sdl(),
                )
            }),
        )
        .route(
            "/docs/react.js",
            get(|| async {
                (
                    [("content-type", "application/javascript")],
                    include_str!("../../../assets/graphiql/react.js"),
                )
            }),
        )
        .route(
            "/docs/react-dom.js",
            get(|| async {
                (
                    [("content-type", "application/javascript")],
                    include_str!("../../../assets/graphiql/react-dom.js"),
                )
            }),
        )
        .route(
            "/docs/graphiql.js",
            get(|| async {
                (
                    [("content-type", "application/javascript")],
                    include_str!("../../../assets/graphiql/graphiql.min.js"),
                )
            }),
        )
        .route(
            "/docs/graphiql.css",
            get(|| async {
                (
                    [("content-type", "text/css")],
                    include_str!("../../../assets/graphiql/graphiql.min.css"),
                )
            }),
        )
        .with_state(state)
}
async fn authorize(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let headers = request.headers();
    let origin_allowed = headers
        .get("origin")
        .is_none_or(|origin| origin == "http://127.0.0.1:47821");
    if !origin_allowed
        || headers
            .get("x-randevu-token")
            .and_then(|value| value.to_str().ok())
            != Some(state.token.as_str())
    {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"errors":[{"message":"Supply the local session token. Only the local docs origin is allowed."}]}))).into_response();
    }
    let span = tracing::info_span!("graphql_request");
    if let Some(parent) = headers
        .get("traceparent")
        .and_then(|value| value.to_str().ok())
    {
        randevu_telemetry::accept_trace_parent(&span, parent);
    }
    next.run(request).instrument(span).await
}
async fn graphql(State(state): State<AppState>, request: GraphQLRequest) -> GraphQLResponse {
    state.schema.execute(request.into_inner()).await.into()
}
async fn diagnostics() -> Json<Vec<randevu_telemetry::LogEntry>> {
    Json(randevu_telemetry::recent_logs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use tower::ServiceExt;
    async fn app() -> Router {
        router(
            Repository::open("sqlite::memory:").await.unwrap(),
            "test-token".into(),
        )
    }
    fn request(body: serde_json::Value) -> Request {
        Request::post("/graphql")
            .header("content-type", "application/json")
            .header("x-randevu-token", "test-token")
            .body(Body::from(body.to_string()))
            .unwrap()
    }
    async fn json(response: Response) -> serde_json::Value {
        serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), 2_000_000)
                .await
                .unwrap(),
        )
        .unwrap()
    }
    #[tokio::test]
    async fn authentication_protects_queries_and_introspection() {
        let app = app().await;
        for (token, origin, expected) in [
            (None, None, 401),
            (Some("wrong"), None, 401),
            (Some("test-token"), Some("https://example.com"), 401),
            (Some("test-token"), Some("http://127.0.0.1:47821"), 200),
            (Some("test-token"), None, 200),
        ] {
            let mut request = Request::post("/graphql").header("content-type", "application/json");
            if let Some(token) = token {
                request = request.header("x-randevu-token", token);
            }
            if let Some(origin) = origin {
                request = request.header("origin", origin);
            }
            let response = app
                .clone()
                .oneshot(
                    request
                        .body(Body::from(
                            serde_json::json!({"query": "{ __schema { queryType { name } } }"})
                                .to_string(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status().as_u16(), expected);
        }
        assert_eq!(
            app.oneshot(
                Request::get("/diagnostics/logs")
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap()
            .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    #[tokio::test]
    async fn schema_exposes_documentation_and_old_rest_is_removed() {
        let app = app().await;
        let result = json(app.clone().oneshot(request(serde_json::json!({"query":"{ __type(name: \"Mutation\") { fields { name description args { name description } } } }"}))).await.unwrap()).await;
        let mutation = result["data"]["__type"]["fields"]
            .as_array()
            .unwrap()
            .iter()
            .find(|field| field["name"] == "saveEvent")
            .unwrap();
        assert!(
            mutation["description"]
                .as_str()
                .unwrap()
                .contains("CONFLICT")
        );
        assert!(mutation["args"][0]["description"].is_string());
        for path in ["/docs", "/schema.graphql", "/docs/graphiql.js"] {
            let response = app
                .clone()
                .oneshot(Request::get(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let bytes = axum::body::to_bytes(response.into_body(), 20_000_000)
                .await
                .unwrap();
            assert!(!String::from_utf8_lossy(&bytes).contains("test-token"));
        }
        for path in ["/api/events", "/openapi.json"] {
            assert_eq!(
                app.clone()
                    .oneshot(Request::get(path).body(Body::empty()).unwrap())
                    .await
                    .unwrap()
                    .status(),
                StatusCode::NOT_FOUND
            );
        }
    }
    #[tokio::test]
    async fn graphql_mutations_remain_idempotent_and_reject_stale_revisions() {
        let app = app().await;
        let event: EventRecord =
            calendar_core::sample_events(chrono::Utc::now().date_naive(), chrono_tz::UTC)
                .remove(0)
                .into();
        let mutation = "mutation Save($event: EventInput!, $operationId: String!, $expectedRevision: Int!) { saveEvent(event:$event,operationId:$operationId,expectedRevision:$expectedRevision) { eventId revision title } }";
        let mut body = serde_json::json!({"query":mutation,"variables":{"event":event,"operationId":"create-1","expectedRevision":-1}});
        let first = json(app.clone().oneshot(request(body.clone())).await.unwrap()).await;
        assert_eq!(first["data"]["saveEvent"]["revision"], 0);
        assert_eq!(
            json(app.clone().oneshot(request(body.clone())).await.unwrap()).await,
            first
        );
        body["variables"]["event"]["title"] = "Changed".into();
        let reused = json(app.clone().oneshot(request(body.clone())).await.unwrap()).await;
        assert_eq!(reused["errors"][0]["extensions"]["code"], "CONFLICT");
        body["variables"]["operationId"] = "update-1".into();
        body["variables"]["expectedRevision"] = 0.into();
        assert_eq!(
            json(app.clone().oneshot(request(body.clone())).await.unwrap()).await["data"]["saveEvent"]
                ["revision"],
            1
        );
        body["variables"]["operationId"] = "stale".into();
        assert_eq!(
            json(app.clone().oneshot(request(body.clone())).await.unwrap()).await["errors"][0]["extensions"]
                ["code"],
            "CONFLICT"
        );
        body["variables"]["event"]["title"] = "".into();
        assert_eq!(
            json(app.oneshot(request(body)).await.unwrap()).await["errors"][0]["extensions"]["code"],
            "INVALID_EVENT"
        );
    }
}
