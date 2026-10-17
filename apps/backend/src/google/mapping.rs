use super::*;

pub(super) fn calendar_api_error(status: u16, body: &serde_json::Value) -> String {
    let reasons: Vec<&str> = ["errors", "details"]
        .into_iter()
        .filter_map(|field| body["error"][field].as_array())
        .flatten()
        .filter_map(|entry| entry["reason"].as_str())
        .collect();
    let has = |expected: &[&str]| reasons.iter().any(|reason| expected.contains(reason));
    if has(&["SERVICE_DISABLED", "accessNotConfigured", "serviceDisabled"]) {
        "Google Calendar API is disabled for your OAuth project. Enable it in Google Cloud > APIs & Services > Library, wait a few minutes, then reconnect.".into()
    } else if has(&["ACCESS_TOKEN_SCOPE_INSUFFICIENT", "insufficientPermissions"]) {
        "Google did not grant the required calendar permission. Reconnect and approve access in Settings.".into()
    } else if status == 401 {
        "Google rejected the access token. Reconnect your account.".into()
    } else if status == 429
        || has(&[
            "rateLimitExceeded",
            "userRateLimitExceeded",
            "quotaExceeded",
            "RATE_LIMIT_EXCEEDED",
        ])
    {
        "Google Calendar rate limit reached. Wait a few minutes before reconnecting.".into()
    } else if status >= 500 {
        "Google Calendar is temporarily unavailable. Try connecting again later.".into()
    } else {
        format!(
            "Google Calendar rejected calendar discovery (HTTP {status}). Check Calendar API enablement, calendar-list consent, and any Workspace administrator restrictions."
        )
    }
}

pub(super) fn provider_operation(value: &serde_json::Value) -> Option<&str> {
    let private = &value["extendedProperties"]["private"];
    private["calendarOperation"]
        .as_str()
        .or_else(|| private["randevuOperation"].as_str())
}

pub(super) fn map_google_event(
    value: &serde_json::Value,
    calendar: &GoogleCalendar,
) -> Result<Option<calendar_core::CalendarEvent>, String> {
    if value["status"] == "cancelled" {
        return Ok(None);
    }
    let all_day = value["start"]["date"].is_string();
    let parse_time = |field: &str| -> Result<chrono::DateTime<chrono::Utc>, String> {
        if all_day {
            let date = value[field]["date"]
                .as_str()
                .ok_or("Missing all-day date.")?
                .parse()
                .map_err(|_| "Invalid all-day date.")?;
            calendar_core::resolve_local(date, 0, chrono_tz::Asia::Jakarta)
        } else {
            value[field]["dateTime"]
                .as_str()
                .ok_or("Missing event time.")?
                .parse::<chrono::DateTime<chrono::Utc>>()
                .map_err(|_| "Invalid Google event time.".into())
        }
    };
    let provider_id = value["id"].as_str().ok_or("Missing Google event ID.")?;
    let start = parse_time("start")?;
    let end = parse_time("end")?;
    if end <= start {
        return Ok(None);
    }
    let timezone = value["start"]["timeZone"]
        .as_str()
        .unwrap_or(&calendar.time_zone);
    let timezone = if timezone.parse::<chrono_tz::Tz>().is_ok() {
        timezone
    } else {
        "Asia/Jakarta"
    };
    Ok(Some(calendar_core::CalendarEvent {
        event_id: format!(
            "google:{}:{}",
            URL_SAFE_NO_PAD.encode(calendar.id.as_bytes()),
            provider_id
        ),
        calendar_id: calendar.id.clone(),
        title: value["summary"].as_str().unwrap_or("Busy").into(),
        start,
        end,
        timezone: timezone.into(),
        all_day,
        description: value["description"].as_str().unwrap_or_default().into(),
        location: value["location"].as_str().unwrap_or_default().into(),
        color: u32::from_str_radix(calendar.background_color.trim_start_matches('#'), 16)
            .unwrap_or(0x6ba5d7),
        revision: value["updated"]
            .as_str()
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.timestamp_millis())
            .unwrap_or(0),
        read_only: !matches!(calendar.access_role.as_str(), "owner" | "writer")
            || value["eventType"]
                .as_str()
                .is_some_and(|kind| kind != "default"),
    }))
}
