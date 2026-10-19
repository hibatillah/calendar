use calendar_core::CalendarEvent;
use chrono::{Duration, NaiveDate, Timelike};
use chrono_tz::Tz;

#[derive(Clone, Debug)]
pub struct EventPlacement {
    pub event_id: String,
    pub day: usize,
    pub start_minute: f32,
    pub end_minute: f32,
    pub column: usize,
    pub column_count: usize,
}

pub fn week_layout(events: &[CalendarEvent], week: NaiveDate, timezone: Tz) -> Vec<EventPlacement> {
    range_layout(events, week, 7, timezone)
}

pub fn range_layout(
    events: &[CalendarEvent],
    first_date: NaiveDate,
    day_count: usize,
    timezone: Tz,
) -> Vec<EventPlacement> {
    let mut result = Vec::new();
    for day in 0..day_count {
        let date = first_date + Duration::days(day as i64);
        let mut segments = Vec::new();
        for event in events.iter().filter(|event| !event.all_day) {
            let start = event.start.with_timezone(&timezone);
            let end = event.end.with_timezone(&timezone);
            if start.date_naive() > date || end.date_naive() < date {
                continue;
            }
            let start_minute = if start.date_naive() < date {
                0.0
            } else {
                (start.hour() * 60 + start.minute()) as f32
            };
            let end_minute = if end.date_naive() > date {
                1440.0
            } else {
                (end.hour() * 60 + end.minute()) as f32
            };
            if end_minute <= start_minute {
                continue;
            }
            segments.push(EventPlacement {
                event_id: event.event_id.clone(),
                day,
                start_minute,
                end_minute,
                column: 0,
                column_count: 1,
            });
        }
        segments.sort_by(|left, right| {
            left.start_minute
                .total_cmp(&right.start_minute)
                .then(left.event_id.cmp(&right.event_id))
        });
        let mut group_start = 0;
        while group_start < segments.len() {
            let mut group_end = group_start + 1;
            let mut latest_end = segments[group_start].end_minute;
            while group_end < segments.len() && segments[group_end].start_minute < latest_end {
                latest_end = latest_end.max(segments[group_end].end_minute);
                group_end += 1;
            }
            let mut column_ends: Vec<f32> = Vec::new();
            for segment in &mut segments[group_start..group_end] {
                let column = column_ends
                    .iter()
                    .position(|end| *end <= segment.start_minute)
                    .unwrap_or(column_ends.len());
                if column == column_ends.len() {
                    column_ends.push(segment.end_minute);
                } else {
                    column_ends[column] = segment.end_minute;
                }
                segment.column = column;
            }
            for segment in &mut segments[group_start..group_end] {
                segment.column_count = column_ends.len();
            }
            group_start = group_end;
        }
        result.extend(segments);
    }
    result
}

pub fn snap_minutes(delta: f32) -> i64 {
    (delta / 15.0).round() as i64 * 15
}

#[cfg(test)]
mod tests {
    use super::*;
    use calendar_core::{monday, sample_events};
    #[test]
    fn overlaps_share_columns_but_adjacent_events_do_not() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap();
        let mut events = sample_events(date, chrono_tz::UTC);
        let placements = week_layout(&events, monday(date), chrono_tz::UTC);
        assert_eq!(
            placements
                .iter()
                .filter(|placement| placement.column_count == 2)
                .count(),
            2
        );
        events[4].start = events[3].end;
        let placements = week_layout(&events, monday(date), chrono_tz::UTC);
        assert!(
            placements
                .iter()
                .all(|placement| placement.column_count == 1)
        );
    }

    #[test]
    fn sliding_range_includes_partial_eighth_day_and_clips_overnight_events() {
        let first_date = NaiveDate::from_ymd_opt(2026, 12, 29).unwrap();
        let mut event = sample_events(first_date, chrono_tz::UTC).remove(0);
        event.start =
            calendar_core::resolve_local(first_date + Duration::days(6), 1380, chrono_tz::UTC)
                .unwrap();
        event.end = event.start + Duration::hours(2);
        let placements = range_layout(&[event.clone()], first_date, 8, chrono_tz::UTC);
        assert_eq!(placements.len(), 2);
        assert_eq!((placements[0].day, placements[1].day), (6, 7));
        assert_eq!(
            (placements[1].start_minute, placements[1].end_minute),
            (0.0, 60.0)
        );
        let placements = range_layout(&[event], first_date + Duration::days(7), 8, chrono_tz::UTC);
        assert_eq!(placements.len(), 1);
        assert_eq!(placements[0].day, 0);
    }

    #[test]
    fn midnight_event_splits_without_empty_end_segment() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 12).unwrap();
        let mut event = sample_events(date, chrono_tz::UTC).remove(0);
        event.start = date.and_hms_opt(23, 0, 0).unwrap().and_utc();
        event.end = event.start + Duration::hours(2);
        let placements = week_layout(&[event], date, chrono_tz::UTC);
        assert_eq!(placements.len(), 2);
        assert_eq!(placements[0].end_minute, 1440.0);
        assert_eq!(placements[1].end_minute, 60.0);
    }
}
