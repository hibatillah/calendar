use super::*;

impl CalendarApp {
    pub(super) fn go_today(&mut self) {
        let today = Utc::now().with_timezone(&self.timezone).date_naive();
        self.date = if self.month_view {
            today
        } else {
            week_start(today, self.preferences.sunday_first)
        };
        self.horizontal_fraction = 0.0;
    }

    pub(super) fn display_month_cells(&self) -> Vec<NaiveDate> {
        let first = self.date.with_day(1).unwrap();
        let start = week_start(first, self.preferences.sunday_first);
        (0..42)
            .map(|offset| start + Duration::days(offset))
            .collect()
    }

    pub(super) fn sidebar_width(&self) -> f32 {
        if self.sidebar_open { SIDEBAR } else { 0.0 }
    }

    pub(super) fn set_month_view(&mut self, month_view: bool, context: &mut Context<Self>) {
        self.month_view = month_view;
        self.view_menu_open = false;
        self.horizontal_fraction = 0.0;
        tracing::info!(
            action = "view_changed",
            mode = if month_view { "month" } else { "week" }
        );
        context.notify();
    }

    pub(super) fn toggle_sidebar(&mut self, context: &mut Context<Self>) {
        self.sidebar_open = !self.sidebar_open;
        tracing::info!(action = "sidebar_toggled", visible = self.sidebar_open);
        context.notify();
    }

    pub(super) fn scroll_dates(&mut self, delta: f32, day_width: f32, context: &mut Context<Self>) {
        if self.gesture.is_some() || !delta.is_finite() {
            return;
        }
        let position = self.horizontal_fraction - delta / day_width;
        let days = position.floor() as i64;
        self.date += Duration::days(days);
        self.horizontal_fraction = position - days as f32;
        if days != 0 {
            tracing::info!(action = "timeline_scrolled", offset_days = days);
        }
        context.notify();
    }

    pub(super) fn scroll_timeline(
        &mut self,
        event: &ScrollWheelEvent,
        window: &mut Window,
        context: &mut Context<Self>,
    ) {
        let delta = event.delta.pixel_delta(px(32.0));
        let horizontal = if event.modifiers.shift && delta.x == px(0.0) {
            delta.y
        } else {
            delta.x
        };
        if event.modifiers.shift || horizontal.abs() > delta.y.abs() {
            context.stop_propagation();
            let day_width =
                ((f32::from(window.viewport_size().width) - self.sidebar_width() - GUTTER) / 7.0)
                    .max(80.0);
            self.scroll_dates(f32::from(horizontal), day_width, context);
        }
    }

    pub(super) fn date_visible(&self, date: NaiveDate) -> bool {
        if self.month_view {
            return date.year() == self.date.year() && date.month() == self.date.month();
        }
        // Follow the seven day-column centers, not a clipped sliver at either edge.
        let first_highlighted_date =
            self.date + Duration::days(self.horizontal_fraction.round() as i64);
        (0..7).contains(&(date - first_highlighted_date).num_days())
    }

    pub(super) fn navigate(&mut self, days: i64, context: &mut Context<Self>) {
        calendar_telemetry::track("control_clicked", "week_navigation");
        self.date = if self.month_view {
            let first = self.date.with_day(1).unwrap();
            if days < 0 {
                (first - Duration::days(1)).with_day(1).unwrap()
            } else {
                (first + Duration::days(32)).with_day(1).unwrap()
            }
        } else {
            self.date + Duration::days(days)
        };
        self.horizontal_fraction = 0.0;
        tracing::info!(action = "week_navigated", offset_days = days);
        context.notify();
    }
}
