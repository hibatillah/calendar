use super::*;

impl CalendarApp {
    /// First and last (exclusive) local date the current view needs events for.
    fn displayed_dates(&self) -> (NaiveDate, NaiveDate) {
        if self.month_view {
            let cells = self.display_month_cells();
            return (cells[0], cells[41] + Duration::days(1));
        }

        (self.date, self.date + Duration::days(8))
    }

    /// Why the view may look empty or stale. Each line is a state the user can act on; a calendar
    /// with nothing to say shows nothing.
    pub(super) fn notices(&self) -> Vec<String> {
        let mut notices: Vec<String> = self.store_error.iter().cloned().collect();
        notices.extend(self.write_notice.clone());
        let waiting = self.snapshot.pending.iter().any(|pending| {
            !self
                .writes
                .iter()
                .any(|write| write.operation_id == pending.operation_id)
        });
        if waiting {
            notices.push("Some changes are waiting for Google to confirm them.".into());
        }
        for account in &self.snapshot.accounts {
            if let Some(error) = &account.error {
                notices.push(format!(
                    "Could not refresh {}. {error} Showing saved events.",
                    account.email
                ));
            }
        }

        if self.snapshot.accounts.is_empty() {
            notices.push(if self.snapshot.configured {
                "Connect a Google account in Settings to see your events.".into()
            } else {
                "Sign-in is not configured in this build.".into()
            });
        } else if let Some((start, end)) = self.snapshot.covered {
            let (first, after_last) = self.displayed_dates();
            let (start, end) = (
                start.with_timezone(&self.timezone).date_naive(),
                end.with_timezone(&self.timezone).date_naive(),
            );
            if first < start || after_last > end {
                notices.push(format!(
                    "Only {} to {} is downloaded. An empty day outside it is unknown, not free.",
                    start.format("%b %-d, %Y"),
                    (end - Duration::days(1)).format("%b %-d, %Y")
                ));
            }
        } else if self.syncing {
            notices.push("Loading your calendars…".into());
        }

        notices
    }
}
