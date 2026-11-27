use chrono::{Timelike, Weekday};
use std::path::PathBuf;

const MINUTES_PER_DAY: u32 = 24 * 60;

/// Settings kept in `preferences.json` inside the data directory. A missing or corrupt file gives the defaults.
#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub sunday_first: bool,
    pub tray_all_day: bool,
    pub working_hours: WorkingHours,
    #[serde(skip)]
    path: Option<PathBuf>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            sunday_first: false,
            tray_all_day: true,
            working_hours: WorkingHours::default(),
            path: None,
        }
    }
}

/// One weekday's working hours as minutes after local midnight; `end_minute` is on the same day.
/// The hours are kept while the day is switched off.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct WorkingDay {
    pub enabled: bool,
    pub start_minute: u32,
    pub end_minute: u32,
}

impl Default for WorkingDay {
    fn default() -> Self {
        Self {
            enabled: false,
            start_minute: 9 * 60,
            end_minute: 17 * 60,
        }
    }
}

impl WorkingDay {
    fn is_valid(&self) -> bool {
        self.start_minute < self.end_minute && self.end_minute < MINUTES_PER_DAY
    }

    /// Reads `HH:MM` text; the end must come after the start on the same day.
    pub fn parse(enabled: bool, start: &str, end: &str) -> Result<Self, String> {
        let minute = |text: &str, label: &str| {
            chrono::NaiveTime::parse_from_str(text.trim(), "%H:%M")
                .map(|time| time.hour() * 60 + time.minute())
                .map_err(|_| format!("Use HH:MM for the {label} time."))
        };
        let day = Self {
            enabled,
            start_minute: minute(start, "start")?,
            end_minute: minute(end, "end")?,
        };
        if day.is_valid() {
            Ok(day)
        } else {
            Err("The end time must be after the start time on the same day.".into())
        }
    }
}

/// The weekly schedule, indexed by days from Monday; it does not depend on the week-start preference.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct WorkingHours {
    /// Whether the week and day views shade the hours outside the schedule.
    pub enabled: bool,
    pub days: [WorkingDay; 7],
}

impl Default for WorkingHours {
    fn default() -> Self {
        let mut days = [WorkingDay::default(); 7];
        for day in &mut days[..5] {
            day.enabled = true;
        }
        Self {
            enabled: true,
            days,
        }
    }
}

impl WorkingHours {
    /// Replaces invalid stored hours with the defaults, so a hand-edited file cannot break the grid.
    pub fn sanitized(mut self) -> Self {
        for day in &mut self.days {
            if !day.is_valid() {
                *day = WorkingDay {
                    enabled: day.enabled,
                    ..WorkingDay::default()
                };
            }
        }
        self
    }

    /// Minute ranges of the day to shade: before the start and after the end, or the whole day when it is off.
    pub fn off_ranges(&self, weekday: Weekday) -> [Option<(u32, u32)>; 2] {
        if !self.enabled {
            return [None, None];
        }
        let day = self.days[weekday.num_days_from_monday() as usize];
        if !day.enabled {
            return [Some((0, MINUTES_PER_DAY)), None];
        }
        [
            (day.start_minute > 0).then_some((0, day.start_minute)),
            (day.end_minute < MINUTES_PER_DAY).then_some((day.end_minute, MINUTES_PER_DAY)),
        ]
    }
}

impl Preferences {
    /// The file in the data directory. Without a data directory preferences last for this run only.
    pub fn load_from_data_directory() -> Self {
        Self::load(
            calendar_sync::data_directory().map(|directory| directory.join("preferences.json")),
        )
    }

    pub fn load(path: Option<PathBuf>) -> Self {
        let mut preferences: Self = path
            .as_ref()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        preferences.working_hours = preferences.working_hours.sanitized();
        preferences.path = path;
        preferences
    }

    pub fn save(&self) {
        let Some(path) = &self.path else { return };
        let written = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(path, serde_json::to_vec(self).unwrap_or_default()));
        if let Err(error) = written {
            tracing::warn!(%error, "Could not save preferences");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_path(name: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!("calendar-preferences-{}", uuid::Uuid::new_v4()))
            .join(name)
    }

    #[test]
    fn missing_and_corrupt_files_give_defaults() {
        let missing = temporary_path("preferences.json");
        assert_eq!(
            Preferences::load(Some(missing.clone())),
            Preferences {
                path: Some(missing),
                ..Default::default()
            }
        );

        let corrupt = temporary_path("preferences.json");
        std::fs::create_dir_all(corrupt.parent().unwrap()).unwrap();
        std::fs::write(&corrupt, "{not json").unwrap();
        let loaded = Preferences::load(Some(corrupt.clone()));
        assert!(!loaded.sunday_first && loaded.tray_all_day);
        std::fs::remove_dir_all(corrupt.parent().unwrap()).unwrap();
    }

    #[test]
    fn saved_values_round_trip_and_unknown_fields_are_ignored() {
        let path = temporary_path("preferences.json");
        let mut preferences = Preferences::load(Some(path.clone()));
        preferences.sunday_first = true;
        preferences.tray_all_day = false;
        preferences.save();

        let loaded = Preferences::load(Some(path.clone()));
        assert!(loaded.sunday_first && !loaded.tray_all_day);

        std::fs::write(&path, r#"{"sunday_first":true,"old_option":1}"#).unwrap();
        let loaded = Preferences::load(Some(path.clone()));
        assert!(loaded.sunday_first && loaded.tray_all_day);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn working_hours_default_to_weekdays_and_round_trip_with_old_files() {
        let path = temporary_path("preferences.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, r#"{"sunday_first":true}"#).unwrap();
        let mut preferences = Preferences::load(Some(path.clone()));
        let hours = preferences.working_hours;
        assert!(hours.enabled && hours.days[..5].iter().all(|day| day.enabled));
        assert!(!hours.days[5].enabled && !hours.days[6].enabled);
        assert_eq!(
            (hours.days[0].start_minute, hours.days[0].end_minute),
            (540, 1020)
        );

        preferences.working_hours.days[5] = WorkingDay::parse(true, "10:30", "14:00").unwrap();
        preferences.save();
        let loaded = Preferences::load(Some(path.clone()));
        assert_eq!(loaded.working_hours, preferences.working_hours);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn working_day_rejects_bad_times_and_ranges_that_do_not_end_later_on_the_same_day() {
        assert!(WorkingDay::parse(true, "09:00", "17:00").is_ok());
        for (start, end) in [("09:00", "09:00"), ("17:00", "09:00"), ("09:00", "24:00")] {
            assert!(
                WorkingDay::parse(true, start, end).is_err(),
                "{start}-{end}"
            );
        }
        for (start, end) in [("nine", "17:00"), ("09:00", ""), ("09:61", "17:00")] {
            let message = WorkingDay::parse(true, start, end).unwrap_err();
            assert!(message.starts_with("Use HH:MM"), "{message}");
        }
    }

    #[test]
    fn stored_hours_that_are_invalid_fall_back_and_off_ranges_follow_the_schedule() {
        let mut hours = WorkingHours::default();
        hours.days[1].start_minute = 900;
        hours.days[1].end_minute = 600;
        hours.days[2].end_minute = 5000;
        let hours = hours.sanitized();
        assert_eq!(
            hours.days[1],
            WorkingDay {
                enabled: true,
                ..WorkingDay::default()
            }
        );
        assert_eq!(hours.days[2].end_minute, 1020);

        assert_eq!(
            hours.off_ranges(Weekday::Mon),
            [Some((0, 540)), Some((1020, 1440))]
        );
        assert_eq!(hours.off_ranges(Weekday::Sat), [Some((0, 1440)), None]);
        let off = WorkingHours {
            enabled: false,
            ..hours
        };
        assert_eq!(off.off_ranges(Weekday::Mon), [None, None]);
    }
}
