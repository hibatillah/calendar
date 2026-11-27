//! What the tray shows, as plain strings built from the cache: pure, so it is tested without Windows.

use calendar_core::CalendarEvent;
use calendar_sync::CachedEvent;
use chrono::{DateTime, TimeZone, Utc};

const TITLE_LIMIT: usize = 40;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    /// Disabled menu item, `Next: <title> · <time>`.
    pub next_item: String,
    /// Icon tooltip; Windows cuts it at 127 characters.
    pub tooltip: String,
    /// Disabled menu item below "Sync now".
    pub last_synced_item: String,
}

/// The first timed event that has not ended yet; `events` must be ordered by start.
pub fn next_event(events: &[CachedEvent], now: DateTime<Utc>) -> Option<&CalendarEvent> {
    events
        .iter()
        .map(|cached| &cached.google.event)
        .find(|event| !event.all_day && event.end > now)
}

pub fn summarize<Zone: TimeZone>(
    events: &[CachedEvent],
    now: DateTime<Utc>,
    last_synced: Option<DateTime<Utc>>,
    zone: &Zone,
) -> Summary
where
    Zone::Offset: std::fmt::Display,
{
    let next = next_event(events, now).map(|event| {
        format!(
            "Next: {} · {}",
            short_title(&event.title),
            time_text(event.start, now, zone)
        )
    });
    Summary {
        tooltip: next
            .as_ref()
            .map_or_else(|| "Calendar".into(), |next| format!("Calendar\n{next}")),
        next_item: next.unwrap_or_else(|| "No upcoming events".into()),
        last_synced_item: last_synced.map_or_else(
            || "Not synced yet".into(),
            |finished| {
                format!(
                    "Last synced {}",
                    finished.with_timezone(zone).format("%H:%M")
                )
            },
        ),
    }
}

fn short_title(title: &str) -> String {
    let title = title.trim();
    if title.is_empty() {
        return "(No title)".into();
    }
    if title.chars().count() <= TITLE_LIMIT {
        return title.into();
    }

    let kept: String = title.chars().take(TITLE_LIMIT - 1).collect();
    format!("{kept}…")
}

fn time_text<Zone: TimeZone>(start: DateTime<Utc>, now: DateTime<Utc>, zone: &Zone) -> String
where
    Zone::Offset: std::fmt::Display,
{
    let local_start = start.with_timezone(zone);
    if start <= now {
        "now".into()
    } else if local_start.date_naive() == now.with_timezone(zone).date_naive() {
        local_start.format("%H:%M").to_string()
    } else {
        local_start.format("%a %d %H:%M").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use calendar_core::CalendarEvent;
    use chrono::{FixedOffset, TimeZone};

    fn zone() -> FixedOffset {
        FixedOffset::east_opt(7 * 3600).unwrap()
    }

    fn at(day: u32, hour: u32, minute: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 3, day, hour, minute, 0).unwrap()
    }

    fn cached(title: &str, start: DateTime<Utc>, end: DateTime<Utc>, all_day: bool) -> CachedEvent {
        CachedEvent {
            account_id: "account".into(),
            google: calendar_google::GoogleEvent {
                event: CalendarEvent {
                    event_id: title.into(),
                    calendar_id: "calendar".into(),
                    title: title.into(),
                    start,
                    end,
                    timezone: "UTC".into(),
                    all_day,
                    description: String::new(),
                    location: String::new(),
                    color: 0,
                    revision: 0,
                    read_only: false,
                },
                etag: String::new(),
                recurring_event_id: None,
                operation_id: None,
            },
        }
    }

    #[test]
    fn next_event_skips_ended_and_all_day_events_and_keeps_one_in_progress() {
        let now = at(10, 3, 0);
        let events = [
            cached("ended", at(10, 1, 0), at(10, 2, 0), false),
            cached("holiday", at(10, 0, 0), at(11, 0, 0), true),
            cached("running", at(10, 2, 30), at(10, 4, 0), false),
            cached("later", at(10, 5, 0), at(10, 6, 0), false),
        ];

        assert_eq!(next_event(&events, now).unwrap().title, "running");
        assert_eq!(next_event(&events[..2], now), None);
    }

    #[test]
    fn labels_show_time_today_weekday_later_and_now_when_started() {
        let now = at(10, 3, 0); // 10:00 local
        let hour = chrono::Duration::hours;
        let today = [cached("Standup", now + hour(2), now + hour(3), false)];
        let later = [cached("Review", at(12, 3, 0), at(12, 4, 0), false)];
        let running = [cached("Call", now - hour(1), now + hour(1), false)];

        let last = Some(at(10, 2, 55));
        assert_eq!(
            summarize(&today, now, last, &zone()),
            Summary {
                next_item: "Next: Standup · 12:00".into(),
                tooltip: "Calendar\nNext: Standup · 12:00".into(),
                last_synced_item: "Last synced 09:55".into(),
            }
        );
        assert_eq!(
            summarize(&later, now, None, &zone()).next_item,
            "Next: Review · Thu 12 10:00"
        );
        assert_eq!(
            summarize(&running, now, None, &zone()).next_item,
            "Next: Call · now"
        );
    }

    #[test]
    fn empty_cache_and_long_titles_stay_readable() {
        let now = at(10, 3, 0);
        let empty = summarize(&[], now, None, &zone());
        assert_eq!(empty.next_item, "No upcoming events");
        assert_eq!(empty.tooltip, "Calendar");
        assert_eq!(empty.last_synced_item, "Not synced yet");

        let long = "x".repeat(100);
        let events = [
            cached(
                &long,
                now + chrono::Duration::hours(1),
                now + chrono::Duration::hours(2),
                false,
            ),
            cached(
                "  ",
                now + chrono::Duration::hours(3),
                now + chrono::Duration::hours(4),
                false,
            ),
        ];
        let summary = summarize(&events, now, None, &zone());
        assert!(summary.tooltip.chars().count() < 128);
        assert!(summary.next_item.contains('…'));
        assert!(short_title("  ") == "(No title)");
    }
}
