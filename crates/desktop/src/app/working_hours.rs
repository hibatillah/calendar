//! The Working hours settings page and the shading it draws behind the week grid.

use super::preferences::WorkingDay;
use super::*;
use gpui_kit::component::input::{Input, InputState};

/// Display name and element identifier of each weekday, indexed by days from Monday.
const DAYS: [(&str, &str); 7] = [
    ("Monday", "working-day-monday"),
    ("Tuesday", "working-day-tuesday"),
    ("Wednesday", "working-day-wednesday"),
    ("Thursday", "working-day-thursday"),
    ("Friday", "working-day-friday"),
    ("Saturday", "working-day-saturday"),
    ("Sunday", "working-day-sunday"),
];

pub(super) struct DayForm {
    pub(super) enabled: bool,
    pub(super) start: Entity<InputState>,
    pub(super) end: Entity<InputState>,
}

/// The unsaved page state; it exists only while the page is open.
pub(super) struct WorkingForm {
    pub(super) marker: bool,
    pub(super) days: Vec<DayForm>,
    /// The result of the last Save: `Ok` when stored, otherwise why not.
    pub(super) status: Option<Result<(), String>>,
}

fn clock(minute: u32) -> String {
    format!("{:02}:{:02}", minute / 60, minute % 60)
}

impl CalendarApp {
    /// Opens a settings page. The page's inputs exist only while it is shown.
    pub(super) fn select_settings_section(
        &mut self,
        section: &str,
        window: &mut Window,
        context: &mut Context<Self>,
    ) {
        self.settings_section = section.into();
        self.working_form =
            (section == "working-hours").then(|| self.new_working_form(window, context));
        context.notify();
    }

    /// Closes Settings and releases the page inputs.
    pub(super) fn close_settings(&mut self) {
        self.settings_open = false;
        self.settings_section = "general".into();
        self.working_form = None;
    }

    fn new_working_form(&self, window: &mut Window, context: &mut Context<Self>) -> WorkingForm {
        let hours = self.preferences.working_hours;
        let mut input = |placeholder: &'static str, minute: u32| {
            context.new(|context| {
                let mut state = InputState::new(window, context).placeholder(placeholder);
                state.set_value(clock(minute), window, context);
                state
            })
        };
        WorkingForm {
            marker: hours.enabled,
            days: hours
                .days
                .iter()
                .map(|day| DayForm {
                    enabled: day.enabled,
                    start: input("09:00", day.start_minute),
                    end: input("17:00", day.end_minute),
                })
                .collect(),
            status: None,
        }
    }

    /// Validates the page, then stores and applies it; a failure changes nothing.
    pub(super) fn save_working_hours(&mut self, context: &mut Context<Self>) {
        let Some(form) = &self.working_form else {
            return;
        };
        let text = |input: &Entity<InputState>| input.read(context).value().to_string();
        let mut hours = self.preferences.working_hours;
        hours.enabled = form.marker;
        let mut result = Ok(());
        for (index, day) in form.days.iter().enumerate() {
            match WorkingDay::parse(day.enabled, &text(&day.start), &text(&day.end)) {
                Ok(parsed) => hours.days[index] = parsed,
                Err(reason) if day.enabled => {
                    result = Err(format!("{}: {reason}", DAYS[index].0));
                    break;
                }
                // A day that is off keeps its stored hours when its fields hold nonsense.
                Err(_) => hours.days[index].enabled = false,
            }
        }
        if result.is_ok() {
            self.preferences.working_hours = hours;
            self.preferences.save();
        }
        if let Some(form) = &mut self.working_form {
            form.status = Some(result);
        }
        context.notify();
    }

    /// Minute ranges of `date` that lie outside the working hours.
    pub(super) fn off_ranges(&self, date: NaiveDate) -> [Option<(u32, u32)>; 2] {
        self.preferences.working_hours.off_ranges(date.weekday())
    }

    /// The shading behind the hour lines, one block per off-hours range of each of the eight columns.
    /// Columns map to dates, so the first weekday never matters for the weekday of a column.
    pub(super) fn off_hours_layer(
        &self,
        first_date: NaiveDate,
        day_width: f32,
        horizontal_offset: f32,
    ) -> Div {
        let minute_height = HOUR / 60.0;
        let mut layer = div()
            .absolute()
            .left(px(GUTTER))
            .top_0()
            .w(px(day_width * 7.0))
            .h_full()
            .overflow_hidden();
        for column in 0..8 {
            let date = first_date + Duration::days(column);
            for (from, to) in self.off_ranges(date).into_iter().flatten() {
                layer = layer.child(
                    div()
                        .absolute()
                        .left(px(column as f32 * day_width - horizontal_offset))
                        .top(px(from as f32 * minute_height))
                        .w(px(day_width))
                        .h(px((to - from) as f32 * minute_height))
                        .bg(rgb(OFF_HOURS)),
                );
            }
        }
        layer
    }

    pub(super) fn working_hours_section(
        &self,
        content: Stateful<Div>,
        context: &mut Context<Self>,
    ) -> Stateful<Div> {
        let heading = div()
            .text_size(px(20.0))
            .font_weight(FontWeight::SEMIBOLD)
            .mb_2()
            .child("Working hours");
        let Some(form) = &self.working_form else {
            return content.child(heading);
        };
        let marker = form.marker;
        let first_day = if self.preferences.sunday_first { 6 } else { 0 };
        let rows = (0..7).map(|offset| {
            let index = (first_day + offset) % 7;
            let day = &form.days[index];
            let enabled = day.enabled;
            div()
                .flex()
                .items_center()
                .gap_3()
                .py_1()
                .child(
                    div()
                        .w(px(100.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .when(!enabled, |element| element.text_color(rgb(MUTED)))
                        .child(DAYS[index].0),
                )
                .child(
                    Self::choice(
                        DAYS[index].1,
                        if enabled { "Working" } else { "Day off" },
                        enabled,
                    )
                    .w(px(84.0))
                    .justify_center()
                    .on_click(context.listener(move |app, _, _, context| {
                        if let Some(day) = app
                            .working_form
                            .as_mut()
                            .and_then(|form| form.days.get_mut(index))
                        {
                            day.enabled = !day.enabled;
                        }
                        context.notify();
                    })),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .when(!enabled, |element| element.opacity(0.5))
                        .child(div().w(px(84.0)).child(Input::new(&day.start)))
                        .child(div().text_color(rgb(MUTED)).child("to"))
                        .child(div().w(px(84.0)).child(Input::new(&day.end))),
                )
        });
        content
            .child(heading)
            .child(Self::setting_row(
                "Mark working hours",
                "Shade the hours outside your working hours in the week view",
                div()
                    .flex()
                    .gap_1()
                    .child(Self::choice("working-marker-on", "Show", marker).on_click(
                        context.listener(|app, _, _, context| {
                            if let Some(form) = &mut app.working_form {
                                form.marker = true;
                            }
                            context.notify();
                        }),
                    ))
                    .child(Self::choice("working-marker-off", "Hide", !marker).on_click(
                        context.listener(|app, _, _, context| {
                            if let Some(form) = &mut app.working_form {
                                form.marker = false;
                            }
                            context.notify();
                        }),
                    )),
            ))
            .child(
                div()
                    .py_2()
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .children(rows),
            )
            .child(
                div()
                    .py_2()
                    .text_size(px(12.0))
                    .text_color(rgb(MUTED))
                    .child("The same schedule repeats every week. It only marks the calendar; it does not affect notifications or invitations."),
            )
            .when_some(form.status.clone(), |element, status| {
                element.child(match status {
                    Ok(()) => note("Saved.", MUTED),
                    Err(reason) => note(reason, DANGER),
                })
            })
            .child(
                button("save-working-hours", "Save")
                    .h(px(32.0))
                    .w(px(80.0))
                    .justify_center()
                    .bg(rgb(ACCENT))
                    .text_color(rgb(ON_ACCENT))
                    .rounded_lg()
                    .on_click(context.listener(|app, _, _, context| app.save_working_hours(context))),
            )
    }
}
