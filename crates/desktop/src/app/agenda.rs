use super::*;

impl CalendarApp {
    pub(super) fn agenda(&self, context: &mut Context<Self>) -> Div {
        let now = Utc::now();
        let today = now.with_timezone(&self.timezone).date_naive();
        let mut events: Vec<_> = self
            .events
            .iter()
            .filter(|event| {
                event.end > now
                    && self.calendar_visible(event)
                    && (self.preferences.tray_all_day || !event.all_day)
            })
            .cloned()
            .collect();
        events.sort_by_key(|event| event.start);
        let today_events: Vec<_> = events
            .iter()
            .filter(|event| event.start.with_timezone(&self.timezone).date_naive() <= today)
            .cloned()
            .collect();
        let shown = if today_events.is_empty() {
            events.into_iter().take(3).collect()
        } else {
            today_events
        };
        div()
            .size_full()
            .p_5()
            .flex()
            .flex_col()
            .gap_3()
            .bg(rgb(0x202020).opacity(0.84))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .child(div().text_size(px(18.0)).child("Coming up"))
                    .child(
                        button("close-agenda", "×").on_click(|_, window, _| window.remove_window()),
                    ),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(rgb(MUTED))
                    .child(format!(
                        "{} · {}",
                        today.format("%A, %d %B"),
                        if self.online {
                            "Local preview"
                        } else {
                            "Offline cache"
                        }
                    )),
            )
            .when(shown.is_empty(), |element| {
                element.child(
                    div()
                        .py_6()
                        .text_color(rgb(MUTED))
                        .child("Nothing coming up in your cached calendar."),
                )
            })
            .children(shown.into_iter().take(6).map(|event| {
                div()
                    .id(SharedString::from(event.event_id.clone()))
                    .p_3()
                    .rounded_lg()
                    .border_l_2()
                    .border_color(rgb(event.color))
                    .bg(rgb(event.color).opacity(0.12))
                    .cursor_pointer()
                    .child(div().text_size(px(12.0)).child(event.title.clone()))
                    .child(
                        div()
                            .mt_1()
                            .text_size(px(11.0))
                            .text_color(rgb(MUTED))
                            .child(
                                event
                                    .start
                                    .with_timezone(&self.timezone)
                                    .format("%a %d · %H:%M")
                                    .to_string(),
                            ),
                    )
                    .on_click(context.listener(move |_, _, window, context| {
                        crate::show_event(event.clone(), context);
                        window.remove_window();
                    }))
            }))
            .child(div().flex_1())
            .child(
                button("open-main", "Open calendar  ↗").on_click(|_, _, context| {
                    context.activate(true);
                    crate::open_main(context);
                }),
            )
    }
}
