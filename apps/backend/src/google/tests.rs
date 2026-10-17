use super::*;
#[tokio::test]
async fn browser_flow_uses_loopback_and_consumes_denial_once() {
    let auth = GoogleAuth {
        slot: "test".into(),
        api_origin: "https://www.googleapis.com".into(),
        config: Some(Config {
            client_id: "test.apps.googleusercontent.com".into(),
            client_secret: "test".into(),
        }),
        state: Arc::new(Mutex::new(State {
            loaded: true,
            ..State::default()
        })),
        client: reqwest::Client::new(),
    };
    let authorization = reqwest::Url::parse(&auth.begin().await.unwrap()).unwrap();
    assert_eq!(authorization.host_str(), Some("accounts.google.com"));
    let parameters: HashMap<_, _> = authorization.query_pairs().into_owned().collect();
    assert_eq!(parameters["code_challenge_method"], "S256");
    assert_eq!(parameters["scope"], SCOPE);
    assert!(!parameters.contains_key("client_secret"));
    let callback = reqwest::Url::parse(&parameters["redirect_uri"]).unwrap();
    assert_eq!(callback.host_str(), Some("127.0.0.1"));
    assert!(callback.port().is_some());
    assert!(auth.begin().await.is_err());
    let response = auth
        .client
        .get(callback)
        .query(&[
            ("state", parameters["state"].as_str()),
            ("error", "access_denied"),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert!(response.text().await.unwrap().contains("did not complete"));
    let status = auth.status().await;
    assert!(!status.connected);
    assert!(!status.connecting);
    assert!(status.message.contains("cancelled"));
}

#[test]
fn calendar_errors_distinguish_disabled_api_from_missing_consent() {
    let disabled = serde_json::json!({"error":{"details":[{"reason":"SERVICE_DISABLED"}], "message":"private provider data"}});
    let permission = serde_json::json!({"error":{"errors":[{"reason":"insufficientPermissions"}]}});
    assert!(calendar_api_error(403, &disabled).contains("disabled"));
    assert!(!calendar_api_error(403, &disabled).contains("private provider data"));
    assert!(calendar_api_error(403, &permission).contains("approve access"));
    assert!(calendar_api_error(429, &serde_json::Value::Null).contains("rate limit"));
    assert!(calendar_api_error(400, &serde_json::Value::Null).contains("HTTP 400"));
}

fn test_calendar(identifier: &str) -> GoogleCalendar {
    GoogleCalendar {
        id: identifier.into(),
        summary: "Test".into(),
        primary: true,
        access_role: "owner".into(),
        background_color: "#123456".into(),
        time_zone: "Asia/Jakarta".into(),
    }
}

#[test]
fn event_mapping_preserves_offsets_and_calendar_identity() {
    let value = serde_json::json!({"id":"recurring_20261015T020000Z", "summary":"Meeting", "start":{"dateTime":"2026-10-15T09:00:00+07:00"}, "end":{"dateTime":"2026-10-15T10:00:00+07:00"}});
    let event = map_google_event(&value, &test_calendar("one"))
        .unwrap()
        .unwrap();
    let other = map_google_event(&value, &test_calendar("two"))
        .unwrap()
        .unwrap();
    assert_eq!(event.start.to_rfc3339(), "2026-10-15T02:00:00+00:00");
    assert_eq!(event.end - event.start, chrono::Duration::hours(1));
    assert_ne!(event.event_id, other.event_id);
    assert!(!event.read_only);
    assert_eq!(event.color, 0x123456);
}

#[test]
fn all_day_mapping_preserves_exclusive_dates_and_skips_cancellations() {
    let value = serde_json::json!({"id":"holiday", "start":{"date":"2026-10-15"}, "end":{"date":"2026-10-17"}});
    let event = map_google_event(&value, &test_calendar("one"))
        .unwrap()
        .unwrap();
    assert!(event.all_day);
    assert_eq!(
        event
            .start
            .with_timezone(&chrono_tz::Asia::Jakarta)
            .date_naive()
            .to_string(),
        "2026-10-15"
    );
    assert_eq!(
        event
            .end
            .with_timezone(&chrono_tz::Asia::Jakarta)
            .date_naive()
            .to_string(),
        "2026-10-17"
    );
    assert_eq!(event.end - event.start, chrono::Duration::days(2));
    assert!(
        map_google_event(
            &serde_json::json!({"status":"cancelled"}),
            &test_calendar("one")
        )
        .unwrap()
        .is_none()
    );
}

#[tokio::test]
async fn google_writes_are_retry_safe_and_reject_stale_edits() {
    use axum::{
        Json,
        extract::State as AxumState,
        http::{HeaderMap, StatusCode},
        routing::post,
    };
    let stored = Arc::new(Mutex::new(serde_json::Value::Null));
    let routes = Router::new()
        .route(
            "/calendar/v3/calendars/one/events",
            post(
                |AxumState(stored): AxumState<Arc<Mutex<serde_json::Value>>>,
                 Json(mut value): Json<serde_json::Value>| async move {
                    let mut stored = stored.lock().await;
                    if !stored.is_null() {
                        return (StatusCode::CONFLICT, Json(stored.clone()));
                    }
                    value["updated"] = "2026-10-15T02:00:00Z".into();
                    value["etag"] = "version-one".into();
                    *stored = value.clone();
                    (StatusCode::OK, Json(value))
                },
            ),
        )
        .route(
            "/calendar/v3/calendars/one/events/{event}",
            get(
                |AxumState(stored): AxumState<Arc<Mutex<serde_json::Value>>>| async move {
                    Json(stored.lock().await.clone())
                },
            )
            .patch(
                |AxumState(stored): AxumState<Arc<Mutex<serde_json::Value>>>,
                 headers: HeaderMap,
                 Json(value): Json<serde_json::Value>| async move {
                    let mut stored = stored.lock().await;
                    assert_eq!(headers.get("if-match").unwrap(), "version-one");
                    for (key, value) in value.as_object().unwrap() {
                        stored[key] = value.clone();
                    }
                    stored["updated"] = "2026-10-15T02:01:00Z".into();
                    stored["etag"] = "version-two".into();
                    Json(stored.clone())
                },
            ),
        )
        .with_state(stored.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, routes).await.unwrap();
    });
    let auth = GoogleAuth {
        slot: "test".into(),
        api_origin: origin,
        config: None,
        client: reqwest::Client::new(),
        state: Arc::new(Mutex::new(State {
            loaded: true,
            credentials: Some(Credentials {
                client_id: "test".into(),
                access_token: "test".into(),
                refresh_token: "test".into(),
                expires_at: chrono::Utc::now().timestamp() + 3600,
                scope: SCOPE.into(),
                name: String::new(),
                email: String::new(),
                picture: String::new(),
            }),
            calendars: vec![test_calendar("one")],
            ..Default::default()
        })),
    };
    let mut event = calendar_core::sample_events(
        chrono::NaiveDate::from_ymd_opt(2026, 10, 15).unwrap(),
        chrono_tz::Asia::Jakarta,
    )
    .remove(0);
    event.event_id = uuid::Uuid::new_v4().to_string();
    event.calendar_id = "one".into();
    let command = calendar_core::EventCommand {
        operation_id: uuid::Uuid::new_v4().to_string(),
        expected_revision: -1,
        event,
    };
    let saved = auth.save(command.clone()).await.unwrap();
    let retry = auth.save(command).await.unwrap();
    assert_eq!(saved.event_id, retry.event_id);
    assert!(!saved.read_only);
    let mut edit = calendar_core::EventCommand {
        operation_id: uuid::Uuid::new_v4().to_string(),
        expected_revision: saved.revision,
        event: saved.clone(),
    };
    edit.event.title = "Updated".into();
    let updated = auth.save(edit.clone()).await.unwrap();
    assert_eq!(updated.title, "Updated");
    assert!(updated.revision > saved.revision);
    assert_eq!(auth.save(edit).await.unwrap().revision, updated.revision);
    let stale = calendar_core::EventCommand {
        operation_id: uuid::Uuid::new_v4().to_string(),
        expected_revision: saved.revision,
        event: saved,
    };
    assert!(
        auth.save(stale)
            .await
            .unwrap_err()
            .contains("changed in Google")
    );
    server.abort();
}

#[test]
fn provider_operation_reads_legacy_markers_but_prefers_calendar() {
    assert_eq!(
        provider_operation(
            &serde_json::json!({"extendedProperties":{"private":{"randevuOperation":"old"}}})
        ),
        Some("old")
    );
    assert_eq!(
        provider_operation(
            &serde_json::json!({"extendedProperties":{"private":{"randevuOperation":"old", "calendarOperation":"new"}}})
        ),
        Some("new")
    );
    assert_eq!(provider_operation(&serde_json::Value::Null), None);
}

#[test]
fn pkce_matches_rfc7636_vector() {
    assert_eq!(
        challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
}

#[tokio::test]
async fn invalid_state_does_not_consume_pending_sign_in() {
    let auth = GoogleAuth {
        slot: "test".into(),
        api_origin: "https://www.googleapis.com".into(),
        config: None,
        state: Arc::new(Mutex::new(State::default())),
        client: reqwest::Client::new(),
    };
    auth.state.lock().await.pending = Some(Pending {
        state: "correct".into(),
        verifier: "test".into(),
        redirect_uri: "http://127.0.0.1:1/".into(),
    });
    assert!(
        auth.complete(HashMap::from([("state".into(), "wrong".into())]))
            .await
            .is_err()
    );
    assert!(auth.state.lock().await.pending.is_some());
    assert_eq!(
        auth.complete(HashMap::from([
            ("state".into(), "correct".into()),
            ("error".into(), "access_denied".into())
        ]))
        .await,
        Ok(false)
    );
    assert!(auth.state.lock().await.pending.is_none());
    assert!(
        auth.complete(HashMap::from([("state".into(), "correct".into())]))
            .await
            .is_err()
    );
}
