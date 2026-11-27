//! Google calendars and events to `calendar-core` types, and core events back to request bodies.

use crate::Error;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use calendar_core::CalendarEvent;
use chrono::{DateTime, Duration, LocalResult, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;

const OPERATION_PROPERTY: &str = "calendarOperation";
const DEFAULT_COLOR: u32 = 0x6ba5d7;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Calendar {
    pub id: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub primary: bool,
    #[serde(default)]
    pub access_role: String,
    #[serde(default)]
    pub background_color: String,
    #[serde(default)]
    pub time_zone: String,
}

impl Calendar {
    pub fn writable(&self) -> bool {
        matches!(self.access_role.as_str(), "owner" | "writer")
    }
}

/// A mapped event plus the provider data the core type has no room for.
#[derive(Clone, Debug, PartialEq)]
pub struct GoogleEvent {
    pub event: CalendarEvent,
    /// Send as `If-Match` on the next write.
    pub etag: String,
    /// Set for expanded instances of a recurring series; the instance keeps its own `event_id`.
    pub recurring_event_id: Option<String>,
    /// The last write operation that touched this event, used to recognise retries.
    pub operation_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EventChange {
    Upsert(Box<GoogleEvent>),
    /// Cancelled, or no longer displayable; drop it from the cache.
    Deleted {
        event_id: String,
    },
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawTime {
    date: Option<String>,
    date_time: Option<String>,
    time_zone: Option<String>,
}

#[derive(Default, Deserialize)]
struct RawExtendedProperties {
    #[serde(default)]
    private: HashMap<String, String>,
}

/// Only the fields the app uses; cancelled tombstones carry little more than `id` and `status`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawEvent {
    id: String,
    #[serde(default)]
    etag: String,
    status: Option<String>,
    summary: Option<String>,
    description: Option<String>,
    location: Option<String>,
    #[serde(default)]
    start: RawTime,
    #[serde(default)]
    end: RawTime,
    updated: Option<String>,
    event_type: Option<String>,
    recurring_event_id: Option<String>,
    #[serde(default)]
    extended_properties: RawExtendedProperties,
}

/// Stable local identifier: namespaced by calendar so shared calendars and recurring instances never collide.
pub(crate) fn event_id_for(calendar_id: &str, provider_id: &str) -> String {
    format!(
        "google:{}:{provider_id}",
        URL_SAFE_NO_PAD.encode(calendar_id)
    )
}

pub(crate) fn provider_id<'a>(calendar: &Calendar, event_id: &'a str) -> Result<&'a str, Error> {
    event_id
        .strip_prefix(&event_id_for(&calendar.id, ""))
        .filter(|provider_id| !provider_id.is_empty())
        .ok_or(Error::InvalidEvent)
}

/// The provider id of an event created from a draft: the draft's UUID without dashes.
pub(crate) fn draft_provider_id(draft_event_id: &str) -> Result<String, Error> {
    uuid::Uuid::parse_str(draft_event_id)
        .map(|uuid| uuid.simple().to_string())
        .map_err(|_| Error::InvalidEvent)
}

/// The `event_id` a draft gets once created, known before the request is sent so an unknown
/// outcome can be looked up later.
pub fn created_event_id(calendar: &Calendar, draft_event_id: &str) -> Result<String, Error> {
    Ok(event_id_for(
        &calendar.id,
        &draft_provider_id(draft_event_id)?,
    ))
}

/// First valid instant of a date in `zone`, also when a DST gap swallows midnight.
fn start_of_day(date: NaiveDate, zone: Tz) -> DateTime<Utc> {
    let mut local = date.and_time(NaiveTime::MIN);

    loop {
        match zone.from_local_datetime(&local) {
            LocalResult::Single(instant) | LocalResult::Ambiguous(instant, _) => {
                return instant.with_timezone(&Utc);
            }
            LocalResult::None => local += Duration::minutes(30),
        }
    }
}

fn instant(time: &RawTime, zone: Tz) -> Result<(DateTime<Utc>, bool), Error> {
    if let Some(date) = &time.date {
        let date = date.parse().map_err(|_| Error::InvalidResponse)?;
        return Ok((start_of_day(date, zone), true));
    }

    let value = time.date_time.as_deref().ok_or(Error::InvalidResponse)?;
    let parsed = DateTime::parse_from_rfc3339(value).map_err(|_| Error::InvalidResponse)?;

    Ok((parsed.with_timezone(&Utc), false))
}

pub(crate) fn map_event(
    raw: RawEvent,
    calendar: &Calendar,
    display_zone: Tz,
) -> Result<EventChange, Error> {
    let event_id = event_id_for(&calendar.id, &raw.id);
    if raw.status.as_deref() == Some("cancelled") {
        return Ok(EventChange::Deleted { event_id });
    }

    let (start, all_day) = instant(&raw.start, display_zone)?;
    let (end, _) = instant(&raw.end, display_zone)?;
    if end <= start {
        return Ok(EventChange::Deleted { event_id });
    }

    let timezone = [
        raw.start.time_zone.as_deref(),
        Some(calendar.time_zone.as_str()),
    ]
    .into_iter()
    .flatten()
    .find(|name| name.parse::<Tz>().is_ok())
    .unwrap_or(display_zone.name())
    .to_owned();
    let revision = raw
        .updated
        .as_deref()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map_or(0, |value| value.timestamp_millis());

    Ok(EventChange::Upsert(Box::new(GoogleEvent {
        event: CalendarEvent {
            event_id,
            calendar_id: calendar.id.clone(),
            title: raw.summary.unwrap_or_else(|| "Busy".into()),
            start,
            end,
            timezone,
            all_day,
            description: raw.description.unwrap_or_default(),
            location: raw.location.unwrap_or_default(),
            color: u32::from_str_radix(calendar.background_color.trim_start_matches('#'), 16)
                .unwrap_or(DEFAULT_COLOR),
            revision,
            read_only: !calendar.writable()
                || raw
                    .event_type
                    .as_deref()
                    .is_some_and(|kind| kind != "default"),
        },
        etag: raw.etag,
        recurring_event_id: raw.recurring_event_id,
        operation_id: raw
            .extended_properties
            .private
            .get(OPERATION_PROPERTY)
            .cloned(),
    })))
}

/// Request body for insert (`provider_id` set) and patch. All-day events are written as dates in the display zone.
pub(crate) fn event_body(
    event: &CalendarEvent,
    display_zone: Tz,
    operation_id: &str,
    provider_id: Option<&str>,
) -> Value {
    let time = |instant: DateTime<Utc>| {
        if event.all_day {
            json!({ "date": instant.with_timezone(&display_zone).date_naive().to_string() })
        } else {
            json!({ "dateTime": instant.to_rfc3339(), "timeZone": event.timezone })
        }
    };
    let mut body = json!({
        "summary": event.title,
        "description": event.description,
        "location": event.location,
        "start": time(event.start),
        "end": time(event.end),
        "extendedProperties": { "private": { OPERATION_PROPERTY: operation_id } },
    });
    if let Some(provider_id) = provider_id {
        body["id"] = provider_id.into();
    }

    body
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono_tz::{America::Havana, America::New_York, Asia::Jakarta, Pacific::Auckland};

    fn calendar(id: &str, role: &str) -> Calendar {
        Calendar {
            id: id.into(),
            summary: "Test".into(),
            primary: true,
            access_role: role.into(),
            background_color: "#123456".into(),
            time_zone: "Asia/Jakarta".into(),
        }
    }

    fn map(value: Value, calendar: &Calendar, zone: Tz) -> EventChange {
        map_event(serde_json::from_value(value).unwrap(), calendar, zone).unwrap()
    }

    fn upsert(change: EventChange) -> GoogleEvent {
        match change {
            EventChange::Upsert(event) => *event,
            other => panic!("expected an upsert, got {other:?}"),
        }
    }

    #[test]
    fn timed_event_keeps_offset_midnight_and_calendar_identity() {
        let value = json!({"id": "series_20261015T170000Z", "etag": "\"e1\"", "summary": "Late",
            "recurringEventId": "series",
            "start": {"dateTime": "2026-10-16T00:00:00+07:00"}, "end": {"dateTime": "2026-10-16T01:00:00+07:00"}});
        let one = upsert(map(value.clone(), &calendar("one", "owner"), Jakarta));
        let two = upsert(map(value, &calendar("two", "owner"), Jakarta));

        assert_eq!(one.event.start.to_rfc3339(), "2026-10-15T17:00:00+00:00");
        assert_eq!(one.event.end - one.event.start, Duration::hours(1));
        assert_ne!(one.event.event_id, two.event.event_id);
        assert!(one.event.event_id.ends_with(":series_20261015T170000Z"));
        assert_eq!(one.recurring_event_id.as_deref(), Some("series"));
        assert_eq!(one.etag, "\"e1\"");
        assert_eq!(one.event.color, 0x123456);
        assert!(!one.event.read_only);
    }

    #[test]
    fn all_day_end_stays_exclusive_and_cancelled_becomes_a_deletion() {
        let value = json!({"id": "holiday", "start": {"date": "2026-10-15"}, "end": {"date": "2026-10-17"}});
        let event = upsert(map(value, &calendar("one", "owner"), Jakarta)).event;

        assert!(event.all_day);
        assert_eq!(event.end - event.start, Duration::days(2));
        assert_eq!(
            event.end.with_timezone(&Jakarta).date_naive().to_string(),
            "2026-10-17"
        );

        let cancelled = map(
            json!({"id": "gone", "status": "cancelled"}),
            &calendar("one", "owner"),
            Jakarta,
        );
        assert_eq!(
            cancelled,
            EventChange::Deleted {
                event_id: event_id_for("one", "gone")
            }
        );
    }

    #[test]
    fn all_day_event_across_dst_is_23_hours_and_round_trips_as_dates() {
        let value =
            json!({"id": "dst", "start": {"date": "2026-03-08"}, "end": {"date": "2026-03-09"}});
        let event = upsert(map(value, &calendar("one", "owner"), New_York)).event;

        assert_eq!(event.end - event.start, Duration::hours(23));
        let body = event_body(&event, New_York, "operation", None);
        assert_eq!(body["start"]["date"], "2026-03-08");
        assert_eq!(body["end"]["date"], "2026-03-09");
    }

    #[test]
    fn all_day_dates_follow_the_zone_through_both_dst_directions() {
        let all_day = |start: &str, end: &str, zone: Tz| {
            let value = json!({"id": "day", "start": {"date": start}, "end": {"date": end}});
            let event = upsert(map(value, &calendar("one", "owner"), zone)).event;
            let body = event_body(&event, zone, "operation", None);
            assert_eq!(body["start"]["date"], start);
            assert_eq!(body["end"]["date"], end, "the end stays exclusive");

            event
        };

        // Auckland springs forward on 2026-09-27: midnight is 12:00 UTC the day before.
        let auckland = all_day("2026-09-27", "2026-09-28", Auckland);
        assert_eq!(auckland.start.to_rfc3339(), "2026-09-26T12:00:00+00:00");
        assert_eq!(auckland.end - auckland.start, Duration::hours(23));
        // New York falls back on 2026-11-01, so that date is 25 hours long.
        let new_york = all_day("2026-11-01", "2026-11-02", New_York);
        assert_eq!(new_york.start.to_rfc3339(), "2026-11-01T04:00:00+00:00");
        assert_eq!(new_york.end - new_york.start, Duration::hours(25));
    }

    #[test]
    fn timed_event_across_spring_forward_keeps_real_duration_and_zone() {
        let value = json!({"id": "gap", "start": {"dateTime": "2026-03-08T01:30:00-05:00", "timeZone": "America/New_York"},
            "end": {"dateTime": "2026-03-08T03:30:00-04:00"}});
        let event = upsert(map(value, &calendar("one", "owner"), Jakarta)).event;

        assert_eq!(event.end - event.start, Duration::hours(1));
        assert_eq!(event.timezone, "America/New_York");
    }

    #[test]
    fn all_day_starts_at_first_valid_instant_when_dst_swallows_midnight() {
        // Havana springs forward at 00:00 on 2026-03-08, so that date begins at 01:00 local.
        let value =
            json!({"id": "cuba", "start": {"date": "2026-03-08"}, "end": {"date": "2026-03-09"}});
        let event = upsert(map(value, &calendar("one", "owner"), Havana)).event;

        assert_eq!(
            event.start.with_timezone(&Havana).date_naive().to_string(),
            "2026-03-08"
        );
        assert_eq!(event.end - event.start, Duration::hours(23));
    }

    #[test]
    fn read_only_for_reader_calendars_special_types_and_unusable_events() {
        let timed = |extra: Value| {
            let mut value = json!({"id": "a", "start": {"dateTime": "2026-10-15T09:00:00Z"}, "end": {"dateTime": "2026-10-15T10:00:00Z"}});
            value
                .as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            value
        };

        assert!(
            upsert(map(timed(json!({})), &calendar("one", "reader"), Jakarta))
                .event
                .read_only
        );
        assert!(
            upsert(map(
                timed(json!({"eventType": "birthday"})),
                &calendar("one", "owner"),
                Jakarta
            ))
            .event
            .read_only
        );
        assert!(
            !upsert(map(
                timed(json!({"eventType": "default"})),
                &calendar("one", "writer"),
                Jakarta
            ))
            .event
            .read_only
        );

        let backwards = json!({"id": "b", "start": {"dateTime": "2026-10-15T10:00:00Z"}, "end": {"dateTime": "2026-10-15T09:00:00Z"}});
        assert!(matches!(
            map(backwards, &calendar("one", "owner"), Jakarta),
            EventChange::Deleted { .. }
        ));
    }

    #[test]
    fn provider_id_requires_the_same_calendar() {
        let id = event_id_for("one", "abc");

        assert_eq!(provider_id(&calendar("one", "owner"), &id), Ok("abc"));
        assert_eq!(
            provider_id(&calendar("two", "owner"), &id),
            Err(Error::InvalidEvent)
        );
    }
}
