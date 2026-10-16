use chrono::{DateTime, Datelike, Duration, LocalResult, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CalendarEvent {
    pub event_id: String,
    pub calendar_id: String,
    pub title: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub timezone: String,
    pub all_day: bool,
    pub description: String,
    pub location: String,
    pub color: u32,
    pub revision: i64,
    pub read_only: bool,
}

impl CalendarEvent {
    pub fn validate(&self) -> Result<(), String> {
        if self.title.trim().is_empty() {
            return Err("Give the event a title.".into());
        }
        if self.title.len() > 1000 || self.description.len() > 16000 {
            return Err("Event text is too long.".into());
        }
        if self.end <= self.start {
            return Err("End must be after start.".into());
        }
        if self.end - self.start > Duration::days(366) {
            return Err("Event is longer than one year.".into());
        }
        self.timezone
            .parse::<Tz>()
            .map_err(|_| "Choose a valid IANA time zone.".to_string())?;
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventCommand {
    pub operation_id: String,
    pub expected_revision: i64,
    pub event: CalendarEvent,
}

pub fn monday(date: NaiveDate) -> NaiveDate {
    date - Duration::days(date.weekday().num_days_from_monday().into())
}

pub fn month_cells(date: NaiveDate) -> Vec<NaiveDate> {
    let first = date.with_day(1).expect("day one exists");
    let start = monday(first);
    (0..42)
        .map(|offset| start + Duration::days(offset))
        .collect()
}

pub fn resolve_local(date: NaiveDate, minutes: u32, timezone: Tz) -> Result<DateTime<Utc>, String> {
    let local_date = date + Duration::days(i64::from(minutes / 1440));
    let time = NaiveTime::from_hms_opt((minutes % 1440) / 60, minutes % 60, 0).unwrap();
    match timezone.from_local_datetime(&local_date.and_time(time)) {
        LocalResult::Single(value) => Ok(value.with_timezone(&Utc)),
        LocalResult::Ambiguous(_, _) => {
            Err("This time occurs twice during daylight saving. Choose an unambiguous time.".into())
        }
        LocalResult::None => {
            Err("This time does not exist during daylight saving. Choose another time.".into())
        }
    }
}

pub fn sample_events(date: NaiveDate, timezone: Tz) -> Vec<CalendarEvent> {
    let week = monday(date);
    [
        (0, 540, 600, "Weekly planning", 0x8c9ef8),
        (0, 780, 870, "Design exploration", 0xc596d8),
        (1, 600, 690, "Focus time", 0x83b9a0),
        (2, 540, 600, "Product check-in", 0x8c9ef8),
        (2, 570, 660, "Calendar interactions", 0xc596d8),
        (3, 780, 870, "Build something good", 0x83b9a0),
        (4, 660, 720, "Coffee with a friend", 0xd7ae78),
        (4, 900, 990, "Week in review", 0x8c9ef8),
        (5, 600, 720, "A little time outside", 0xd7ae78),
    ]
    .into_iter()
    .map(|(day, start, end, title, color)| CalendarEvent {
        event_id: uuid::Uuid::new_v4().to_string(),
        calendar_id: if color == 0xd7ae78 {
            "personal"
        } else {
            "work"
        }
        .into(),
        title: title.into(),
        start: resolve_local(week + Duration::days(day), start, timezone).unwrap(),
        end: resolve_local(week + Duration::days(day), end, timezone).unwrap(),
        timezone: timezone.name().into(),
        all_day: false,
        description: "Sample event — changes stay on this machine.".into(),
        location: String::new(),
        color,
        revision: 0,
        read_only: false,
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn month_includes_leap_day_and_six_weeks() {
        let cells = month_cells(NaiveDate::from_ymd_opt(2028, 2, 10).unwrap());
        assert_eq!(cells.len(), 42);
        assert_eq!(cells[0].weekday(), chrono::Weekday::Mon);
        assert!(cells.contains(&NaiveDate::from_ymd_opt(2028, 2, 29).unwrap()));
    }

    #[test]
    fn rejects_dst_gap_and_ambiguous_hour() {
        let zone = chrono_tz::America::New_York;
        assert!(resolve_local(NaiveDate::from_ymd_opt(2026, 3, 8).unwrap(), 150, zone).is_err());
        assert!(resolve_local(NaiveDate::from_ymd_opt(2026, 11, 1).unwrap(), 90, zone).is_err());
    }

    #[test]
    fn midnight_rolls_to_next_day() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap();
        assert_eq!(
            resolve_local(date, 1440, chrono_tz::UTC)
                .unwrap()
                .date_naive(),
            date.succ_opt().unwrap()
        );
    }
}
