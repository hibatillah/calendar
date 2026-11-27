use super::*;

impl CalendarApp {
    pub(super) fn show_details(&mut self, event: CalendarEvent, context: &mut Context<Self>) {
        self.selected = Some(event);
        context.notify();
    }

    /// Account email and calendar name for a view key, for the details popover.
    pub(super) fn calendar_label(&self, calendar_key: &str) -> String {
        self.snapshot
            .accounts
            .iter()
            .find_map(|account| {
                account
                    .calendars
                    .iter()
                    .find(|calendar| calendar.key == calendar_key)
                    .map(|calendar| format!("{} · {}", calendar.summary, account.email))
            })
            .unwrap_or_default()
    }

    pub(super) fn details(&self, context: &mut Context<Self>) -> impl IntoElement {
        let selected = self.selected.as_ref().unwrap();
        // Show the placement while saving, not the version the popover was opened with.
        let current = self.events();
        let event = current
            .iter()
            .find(|event| event.event_id == selected.event_id)
            .unwrap_or(selected);
        let start = event.start.with_timezone(&self.timezone);
        let end = event.end.with_timezone(&self.timezone);
        let when = if event.all_day {
            let last_day = end.date_naive() - Duration::days(1);
            if last_day > start.date_naive() {
                format!(
                    "{} – {}",
                    start.format("%a %b %-d"),
                    last_day.format("%a %b %-d")
                )
            } else {
                start.format("%A, %B %-d").to_string()
            }
        } else if start.date_naive() == end.date_naive() {
            format!(
                "{} · {} – {}",
                start.format("%A, %B %-d"),
                start.format("%H:%M"),
                end.format("%H:%M")
            )
        } else {
            format!(
                "{} – {}",
                start.format("%a %b %-d %H:%M"),
                end.format("%a %b %-d %H:%M")
            )
        };
        let row = |label: &'static str, value: String| {
            div()
                .mb_3()
                .child(
                    div()
                        .text_size(px(9.0))
                        .text_color(rgb(MUTED))
                        .mb_1()
                        .child(label),
                )
                .child(div().child(value))
        };

        let saving = self.is_saving(&event.event_id);
        let writable = self.can_write(event);
        let (edit, delete) = (event.clone(), event.clone());

        popover(
            div()
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .items_center()
                        .mb_5()
                        .child("Event details")
                        .child(button("close-editor", "×").on_click(context.listener(
                            |app, _, window, context| {
                                app.selected = None;
                                app.focus.focus(window, context);
                                context.notify();
                            },
                        ))),
                )
                .child(row("TITLE", event.title.clone()))
                .child(row("WHEN", when))
                .when(!event.location.is_empty(), |element| {
                    element.child(row("LOCATION", event.location.clone()))
                })
                .when(!event.description.is_empty(), |element| {
                    element.child(row("NOTES", event.description.clone()))
                })
                .child(row("CALENDAR", self.calendar_label(&event.calendar_id)))
                .when(
                    self.snapshot.recurring.contains(&event.event_id),
                    |element| element.child(note(RECURRING_NOTE, MUTED)),
                )
                .when(!writable, |element| {
                    element.child(note(calendar_sync::WriteError::ReadOnly.to_string(), MUTED))
                })
                .when(saving, |element| {
                    element.child(note(
                        if self
                            .writes
                            .iter()
                            .any(|write| write.unknown && write.event_key == event.event_id)
                        {
                            "This change may have been saved. Waiting for Google to confirm."
                        } else {
                            "Saving…"
                        },
                        MUTED,
                    ))
                })
                .when(writable && !saving, |element| {
                    element.child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                button("edit-event", "Edit")
                                    .h(px(32.0))
                                    .bg(rgb(ACCENT))
                                    .text_color(rgb(ON_ACCENT))
                                    .rounded_lg()
                                    .on_click(context.listener(move |app, _, window, context| {
                                        app.open_editor(edit.clone(), false, window, context)
                                    })),
                            )
                            .child(
                                button("delete-event", "Delete")
                                    .h(px(32.0))
                                    .text_color(rgb(DANGER))
                                    .on_click(context.listener(move |app, _, _, context| {
                                        app.delete_event(delete.clone(), context)
                                    })),
                            ),
                    )
                }),
        )
    }
}
