#![cfg_attr(test, allow(dead_code))]
use calendar_core::{CalendarEvent, EventCommand};
use chrono::Datelike;
use serde_json::{Value, json};
const FIELDS: &str = "event_id:eventId calendar_id:calendarId title start end timezone all_day:allDay description location color revision read_only:readOnly";
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

#[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GoogleConnection {
    pub configured: bool,
    pub connected: bool,
    pub event_access: bool,
    pub account_key: String,
    pub connecting: bool,
    pub message: String,
    pub calendars: Vec<GoogleCalendar>,
    #[serde(default)]
    pub accounts: Vec<GoogleAccount>,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleAccount {
    pub id: String,
    pub name: String,
    pub email: String,
    pub picture: String,
    pub event_access: bool,
    pub write_access: bool,
    pub connecting: bool,
    pub message: String,
    pub calendars: Vec<GoogleCalendar>,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleCalendar {
    pub id: String,
    pub summary: String,
    pub primary: bool,
    pub background_color: String,
    pub access_role: String,
}
impl GoogleCalendar {
    pub fn color(&self) -> u32 {
        u32::from_str_radix(self.background_color.trim_start_matches('#'), 16).unwrap_or(0x6ba5d7)
    }
}
const GOOGLE_FIELDS: &str = "configured connected connecting eventAccess accountKey message calendars { id summary primary backgroundColor accessRole } accounts { id name email picture eventAccess writeAccess connecting message calendars { id summary primary backgroundColor accessRole } }";

pub enum Request {
    ConnectGoogle(Option<String>),
    DisconnectGoogle(Option<String>),
    Refresh,
    ViewMonth(chrono::NaiveDate),
    Save(Box<EventCommand>),
}
pub enum Update {
    GoogleConnection(GoogleConnection),
    GoogleError(String),
    SyncStatus(bool, Option<String>),
    AuthorizationUrl(String),
    Diagnostics(Vec<calendar_telemetry::LogEntry>),
    Snapshot(Vec<CalendarEvent>, bool),
    Saved(CalendarEvent),
    Failed(String),
    Status(bool),
}
pub struct Connection {
    pub sender: Sender<Request>,
    pub receiver: Receiver<Update>,
}

pub fn start() -> Connection {
    let (sender, requests) = mpsc::channel();
    let (updates, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().expect("worker runtime");
        runtime.block_on(async move {
            let _ = std::fs::create_dir_all(".local");
            let cache = match sqlx::SqlitePool::connect("sqlite:.local/desktop.db?mode=rwc").await {
                Ok(cache) => cache,
                Err(error) => { let _ = updates.send(Update::Failed(format!("Cache unavailable: {error}"))); return; }
            };
            if let Err(error) = sqlx::raw_sql("CREATE TABLE IF NOT EXISTS cache (key TEXT PRIMARY KEY, value TEXT NOT NULL); CREATE TABLE IF NOT EXISTS journal (operation_id TEXT PRIMARY KEY, payload TEXT NOT NULL, status TEXT NOT NULL);").execute(&cache).await {
                let _ = updates.send(Update::Failed(format!("Cache setup failed: {error}"))); return;
            }
            // Retire sample cache and pending preview writes; Google caches use account/range keys.
            let _ = sqlx::query("DELETE FROM cache WHERE key='events'").execute(&cache).await;
            let _ = sqlx::query("DELETE FROM journal WHERE json_extract(payload, '$.event.calendar_id') IN ('work','personal')").execute(&cache).await;
            let mut connection: GoogleConnection = match sqlx::query_scalar::<_, String>("SELECT value FROM cache WHERE key='google-connection'").fetch_optional(&cache).await {
                Ok(Some(value)) => serde_json::from_str(&value).unwrap_or_default(), _ => GoogleConnection::default(),
            };
            let _ = updates.send(Update::GoogleConnection(connection.clone()));
            let mut month = chrono::Utc::now().with_timezone(&chrono_tz::Asia::Jakarta).date_naive().with_day(1).unwrap();
            load_cached_range(&cache, &connection.account_key, month, &updates).await;
            let client = reqwest::Client::builder().timeout(Duration::from_secs(90)).build().unwrap();
            let mut next_request = Some(Request::Refresh);
            loop {
                let request = match next_request.take() {
                    Some(request) => request,
                    None => match requests.recv_timeout(Duration::from_secs(if connection.connecting { 2 } else { 30 })) {
                        Ok(request) => request,
                        Err(mpsc::RecvTimeoutError::Timeout) => Request::Refresh,
                        Err(_) => break,
                    }
                };
                match request {
                    Request::ViewMonth(date) => {
                        month = date.with_day(1).unwrap();
                        load_cached_range(&cache, &connection.account_key, month, &updates).await;
                        next_request = Some(Request::Refresh);
                    }
                    Request::ConnectGoogle(account_id) => {
                        match query(&client, json!({"query": "mutation($account:String) { connectGoogle(accountId:$account) }", "variables":{"account":account_id}})).await {
                            Ok(response) => if let Some(url) = response["connectGoogle"].as_str() { let _ = updates.send(Update::AuthorizationUrl(url.into())); },
                            Err(error) => { let _ = updates.send(Update::GoogleError(error)); }
                        }
                        next_request = Some(Request::Refresh);
                    }
                    Request::DisconnectGoogle(account_id) => {
                        if let Err(error) = query(&client, json!({"query": "mutation($account:String) { disconnectGoogle(accountId:$account) }", "variables":{"account":account_id}})).await {
                            let _ = updates.send(Update::GoogleError(error));
                        } else {
                            connection = GoogleConnection::default();
                            let _ = sqlx::query("DELETE FROM cache WHERE key LIKE 'google-%'").execute(&cache).await;
                            let _ = updates.send(Update::Snapshot(Vec::new(), true));
                        }
                        next_request = Some(Request::Refresh);
                    }
                    Request::Refresh => {
                        match query(&client, json!({"query": format!("{{ googleConnection {{ {GOOGLE_FIELDS} }} }}")})).await {
                            Ok(response) => if let Ok(mut current) = serde_json::from_value::<GoogleConnection>(response["googleConnection"].clone()) {
                                if current.connected && current.account_key.is_empty() {
                                    let _ = updates.send(Update::Status(false));
                                    let _ = updates.send(Update::SyncStatus(false, Some(current.message)));
                                    continue;
                                }
                                if connection.account_key != current.account_key || !current.connected {
                                    let _ = sqlx::query("DELETE FROM cache WHERE key LIKE 'google-events:%'").execute(&cache).await;
                                    let _ = updates.send(Update::Snapshot(Vec::new(), true));
                                }
                                cache_profile_images(&client, &mut current).await;
                                connection = current;
                                let _ = sqlx::query("INSERT INTO cache VALUES ('google-connection',?) ON CONFLICT(key) DO UPDATE SET value=excluded.value").bind(serde_json::to_string(&connection).unwrap()).execute(&cache).await;
                                let _ = updates.send(Update::GoogleConnection(connection.clone()));
                            },
                            Err(_) => { let _ = updates.send(Update::Status(false)); continue; }
                        }
                        if let Ok(token) = std::fs::read_to_string(".local/session-token")
                            && let Ok(response) = client.get("http://127.0.0.1:47821/diagnostics/logs").header("x-calendar-token", token.trim()).send().await
                            && response.status().is_success()
                            && let Ok(records) = response.json().await {
                            let _ = updates.send(Update::Diagnostics(records));
                        }
                        if !connection.connected || !connection.event_access {
                            let _ = updates.send(Update::Snapshot(Vec::new(), true));
                            continue;
                        }
                        let _ = updates.send(Update::SyncStatus(true, None));
                        let start_date = month - chrono::Duration::days(7);
                        let end_date = month + chrono::Duration::days(49);
                        let body = json!({"query": format!("query Events($start:String!,$end:String!) {{ events(startDate:$start,endDate:$end) {{ {FIELDS} }} }}"), "variables":{"start":start_date.to_string(),"end":end_date.to_string()}});
                        match query(&client, body).await {
                            Ok(response) => match serde_json::from_value::<Vec<CalendarEvent>>(response["events"].clone()) {
                                Ok(events) => {
                                    let _ = sqlx::query("INSERT INTO cache VALUES (?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value")
                                        .bind(range_key(&connection.account_key, month)).bind(serde_json::to_string(&events).unwrap()).execute(&cache).await;
                                    let _ = updates.send(Update::Snapshot(events, true));
                                    let _ = updates.send(Update::SyncStatus(false, None));
                                },
                                Err(_) => { let _ = updates.send(Update::SyncStatus(false, Some("Invalid calendar response.".into()))); }
                            },
                            Err(error) => { let _ = updates.send(Update::Status(false)); let _ = updates.send(Update::SyncStatus(false, Some(error))); }
                        }
                    }
                    Request::Save(command) => {
                        let payload = serde_json::to_string(&command).unwrap();
                        if sqlx::query("INSERT OR REPLACE INTO journal VALUES (?, ?, 'pending')").bind(&command.operation_id).bind(payload).execute(&cache).await.is_err() {
                            let _ = updates.send(Update::Failed("Could not journal this change; nothing was sent.".into())); continue;
                        }
                        let event = &command.event;
                        let input = json!({"eventId":event.event_id,"calendarId":event.calendar_id,"title":event.title,
                            "start":event.start.to_rfc3339(),"end":event.end.to_rfc3339(),"timezone":event.timezone,
                            "allDay":event.all_day,"description":event.description,"location":event.location,
                            "color":event.color,"revision":event.revision,"readOnly":event.read_only});
                        let result = query(&client, json!({"query":format!("mutation Save($operationId:String!,$expectedRevision:Int!,$event:EventInput!) {{ saveEvent(operationId:$operationId,expectedRevision:$expectedRevision,event:$event) {{ {FIELDS} }} }}"),
                            "variables":{"operationId":command.operation_id,"expectedRevision":command.expected_revision,"event":input}})).await;
                        match result {
                            Ok(response) => match serde_json::from_value::<CalendarEvent>(response["saveEvent"].clone()) {
                                Ok(event) => {
                                    let _ = sqlx::query("UPDATE journal SET status='confirmed' WHERE operation_id=?").bind(&command.operation_id).execute(&cache).await;
                                    let _ = updates.send(Update::Saved(event));
                                }
                                Err(_) => { let _ = updates.send(Update::Failed("Unexpected server response. Refresh before retrying.".into())); }
                            },
                            Err(error) => {
                                let _ = sqlx::query("UPDATE journal SET status='needs_reconciliation' WHERE operation_id=?").bind(&command.operation_id).execute(&cache).await;
                                tracing::warn!(action="mutation_failed", operation_id=%command.operation_id, "Change needs reconciliation");
                                let _ = updates.send(Update::Failed(error));
                            }
                        }
                        next_request = Some(Request::Refresh);
                    }
                }
            }
        });
    });
    Connection { sender, receiver }
}

fn range_key(account_key: &str, month: chrono::NaiveDate) -> String {
    format!("google-events:{account_key}:{month}")
}
async fn load_cached_range(
    cache: &sqlx::SqlitePool,
    account_key: &str,
    month: chrono::NaiveDate,
    updates: &Sender<Update>,
) {
    let events = match sqlx::query_scalar::<_, String>("SELECT value FROM cache WHERE key=?")
        .bind(range_key(account_key, month))
        .fetch_optional(cache)
        .await
    {
        Ok(Some(value)) => serde_json::from_str(&value).unwrap_or_default(),
        _ => Vec::new(),
    };
    let _ = updates.send(Update::Snapshot(events, false));
}

#[cfg(test)]
pub fn disconnected() -> Connection {
    let (sender, _) = mpsc::channel();
    let (_, receiver) = mpsc::channel();
    Connection { sender, receiver }
}

#[tracing::instrument(skip_all, name = "backend_request")]
async fn query(client: &reqwest::Client, body: Value) -> Result<Value, String> {
    let token = std::fs::read_to_string(".local/session-token")
        .map_err(|_| "Start the local backend to edit events.".to_string())?;
    let mut request = client
        .post("http://127.0.0.1:47821/graphql")
        .header("x-calendar-token", token.trim());
    request = request.json(&body);
    for (key, value) in calendar_telemetry::outgoing_trace_headers() {
        request = request.header(key, value);
    }
    let response = request
        .send()
        .await
        .map_err(|_| "Backend unavailable. Cached calendar is still readable.".to_string())?;
    let status = response.status();
    let response: Value = response
        .json()
        .await
        .map_err(|_| "Invalid backend response".to_string())?;
    if !status.is_success() {
        return Err(response["message"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("Backend returned {status}")));
    }
    if let Some(error) = response["errors"]
        .as_array()
        .and_then(|errors| errors.first())
    {
        return Err(error["message"]
            .as_str()
            .unwrap_or("Unable to complete calendar operation")
            .into());
    }
    Ok(response["data"].clone())
}

async fn cache_profile_images(client: &reqwest::Client, connection: &mut GoogleConnection) {
    let Ok(directory) = std::env::current_dir().map(|path| path.join(".local/avatars")) else {
        return;
    };
    if std::fs::create_dir_all(&directory).is_err() {
        return;
    }
    for account in &mut connection.accounts {
        if account.id != "desktop-account" && uuid::Uuid::parse_str(&account.id).is_err() {
            account.picture.clear();
            continue;
        }
        let path = directory.join(format!("{}.png", account.id));
        if !path.exists() {
            let valid = reqwest::Url::parse(&account.picture).is_ok_and(|url| {
                url.scheme() == "https"
                    && url
                        .host_str()
                        .is_some_and(|host| host.ends_with(".googleusercontent.com"))
            });
            if valid
                && let Ok(response) = client
                    .get(&account.picture)
                    .timeout(Duration::from_secs(5))
                    .send()
                    .await
                && response.status().is_success()
                && let Ok(bytes) = response.bytes().await
                && bytes.len() < 2_000_000
            {
                let _ = std::fs::write(&path, bytes);
            }
        }
        account.picture = if path.exists() {
            path.to_string_lossy().into_owned()
        } else {
            String::new()
        };
    }
}
