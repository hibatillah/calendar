use super::*;

impl CalendarApp {
    pub(super) fn week_grid(
        &self,
        window: &Window,
        context: &mut Context<Self>,
    ) -> impl IntoElement {
        let day_width = self.day_width(window);
        let width = day_width * 7.0;
        let first_date = self.date;
        let visible: Vec<_> = self
            .events()
            .iter()
            .filter(|event| self.calendar_visible(event))
            .cloned()
            .collect();
        let placements = range_layout(&visible, first_date, 8, self.timezone);
        let mut grid = div().relative().w(px(width + GUTTER)).h(px(24.0 * HOUR));
        grid = grid.child(self.off_hours_layer(
            first_date,
            day_width,
            self.horizontal_fraction * day_width,
        ));
        for hour in 0..24 {
            grid = grid.child(
                div()
                    .absolute()
                    .top(px(hour as f32 * HOUR))
                    .left_0()
                    .w_full()
                    .h(px(HOUR))
                    .border_t_1()
                    .border_color(rgb(0xffffff).opacity(0.055))
                    .child(
                        div()
                            .w(px(GUTTER - 8.0))
                            .text_right()
                            .pt_1()
                            .text_size(px(10.0))
                            .text_color(rgb(MUTED))
                            .child(format!("{hour:02}:00")),
                    ),
            );
        }
        let horizontal_offset = self.horizontal_fraction * day_width;
        let mut days_grid = div()
            .absolute()
            .left(px(GUTTER))
            .top_0()
            .w(px(width))
            .h_full()
            .overflow_hidden();
        for day in 0..8 {
            days_grid = days_grid.child(
                div()
                    .id(("day-grid", day as usize))
                    .absolute()
                    .left(px(day as f32 * day_width - horizontal_offset))
                    .top_0()
                    .w(px(day_width))
                    .h_full()
                    .border_l_1()
                    .border_color(rgb(0xffffff).opacity(0.06))
                    .on_mouse_down(
                        MouseButton::Left,
                        context.listener(|app, event: &MouseDownEvent, window, context| {
                            app.begin_create(event.position, window, context)
                        }),
                    ),
            );
        }
        for placement in placements {
            let Some(event) = visible
                .iter()
                .find(|event| event.event_id == placement.event_id)
                .cloned()
            else {
                continue;
            };
            let event_width = (day_width - 7.0) / placement.column_count as f32;
            let height = ((placement.end_minute - placement.start_minute) * HOUR / 60.0).max(22.0);
            let drag_event = event.clone();
            let resize_event = event.clone();
            let saving = self.is_saving(&event.event_id);
            days_grid = days_grid.child(
                div()
                    .id(SharedString::from(format!(
                        "{}-{}",
                        event.event_id,
                        first_date + Duration::days(placement.day as i64)
                    )))
                    .absolute()
                    .left(px(placement.day as f32 * day_width - horizontal_offset
                        + placement.column as f32 * event_width
                        + 3.0))
                    .top(px(placement.start_minute * HOUR / 60.0))
                    .w(px(event_width - 2.0
                        + if placement.column + 1 < placement.column_count {
                            (event_width * 0.18).min(22.0)
                        } else {
                            0.0
                        }))
                    .h(px(height))
                    .rounded_md()
                    .overflow_hidden()
                    .bg(rgb(event_surface(event.color)))
                    .border_l_2()
                    .border_color(rgb(event.color))
                    .cursor_pointer()
                    .when(saving, |element| element.opacity(0.6))
                    .hover(|style| style.bg(rgb(event_surface(event.color))))
                    .on_mouse_down(
                        MouseButton::Left,
                        context.listener(move |app, event: &MouseDownEvent, _, context| {
                            context.stop_propagation();
                            app.begin_gesture(
                                GestureKind::Move,
                                drag_event.clone(),
                                event.position,
                                context,
                            )
                        }),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("resize-{}", event.event_id)))
                            .absolute()
                            .bottom_0()
                            .left_0()
                            .w_full()
                            .h(px(6.0))
                            .cursor_row_resize()
                            .on_mouse_down(
                                MouseButton::Left,
                                context.listener(move |app, event: &MouseDownEvent, _, context| {
                                    context.stop_propagation();
                                    app.begin_gesture(
                                        GestureKind::Resize,
                                        resize_event.clone(),
                                        event.position,
                                        context,
                                    )
                                }),
                            ),
                    )
                    .child(
                        div()
                            .px_1()
                            .py_1()
                            .text_size(px(11.0))
                            .text_color(rgb(0xf0e7e5))
                            .font_weight(FontWeight::MEDIUM)
                            .child(event.title.clone())
                            .when(height > 42.0, |element| {
                                element.child(
                                    div()
                                        .mt_0()
                                        .text_color(rgb(event.color))
                                        .text_size(px(10.0))
                                        .font_weight(FontWeight::NORMAL)
                                        .child(if saving {
                                            "Saving…".to_owned()
                                        } else {
                                            format!(
                                                "{} – {}",
                                                event
                                                    .start
                                                    .with_timezone(&self.timezone)
                                                    .format("%H:%M"),
                                                event
                                                    .end
                                                    .with_timezone(&self.timezone)
                                                    .format("%H:%M")
                                            )
                                        }),
                                )
                            }),
                    ),
            );
        }
        let now = Utc::now().with_timezone(&self.timezone);
        let today_offset = (now.date_naive() - first_date).num_days();
        if (0..8).contains(&today_offset) {
            days_grid = days_grid.child(
                div()
                    .absolute()
                    .left(px(today_offset as f32 * day_width - horizontal_offset))
                    .top(px((now.hour() * 60 + now.minute()) as f32 * HOUR / 60.0))
                    .w(px(day_width))
                    .h(px(1.0))
                    .bg(rgb(0xe8a69a)),
            );
        }
        div()
            .id("week-scroll")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .child(
                grid.child(days_grid)
                    .on_scroll_wheel(context.listener(Self::scroll_timeline)),
            )
    }

    pub(super) fn month_grid(&self, context: &mut Context<Self>) -> impl IntoElement {
        let cells = self.display_month_cells();
        let today = Utc::now().with_timezone(&self.timezone).date_naive();
        let all_events = self.events();
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(
                div().h(px(26.0)).flex().flex_shrink_0().children(
                    (if self.preferences.sunday_first {
                        ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
                    } else {
                        ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
                    })
                    .into_iter()
                    .map(|day| {
                        div()
                            .flex_1()
                            .text_center()
                            .text_color(rgb(MUTED))
                            .child(day)
                    }),
                ),
            )
            .children(cells.chunks(7).enumerate().map(|(row, dates)| {
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .children(dates.iter().enumerate().map(|(column, date)| {
                        let date = *date;
                        let mut events: Vec<_> = all_events
                            .iter()
                            .filter(|event| {
                                (self.calendar_visible(event))
                                    && event.start.with_timezone(&self.timezone).date_naive()
                                        <= date
                                    && (event.end - Duration::nanoseconds(1))
                                        .with_timezone(&self.timezone)
                                        .date_naive()
                                        >= date
                            })
                            .collect();
                        events.sort_by_key(|event| (!event.all_day, event.start));
                        div()
                            .id(("month-cell", row * 7 + column))
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .overflow_hidden()
                            .flex()
                            .flex_col()
                            .border_t_1()
                            .border_l_1()
                            .border_color(rgb(0x242424))
                            .child(
                                div().flex().justify_end().p_1().flex_shrink_0().child(
                                    div()
                                        .px_1()
                                        .h(px(22.0))
                                        .rounded_md()
                                        .text_size(px(12.0))
                                        .text_color(rgb(if date.month() == self.date.month() {
                                            INK
                                        } else {
                                            0x626262
                                        }))
                                        .when(date == today, |element| {
                                            element.bg(rgb(0xf45151)).text_color(rgb(0xffffff))
                                        })
                                        .child(if date.day() == 1 {
                                            date.format("%B %-d").to_string()
                                        } else {
                                            date.day().to_string()
                                        }),
                                ),
                            )
                            .child(
                                div()
                                    .id(("month-events", row * 7 + column))
                                    .flex_1()
                                    .min_h_0()
                                    .overflow_y_scroll()
                                    .children(events.into_iter().map(|event| {
                                        let event = event.clone();
                                        div()
                                            .id(SharedString::from(format!(
                                                "month-{}-{date}",
                                                event.event_id
                                            )))
                                            .mx_1()
                                            .mb_1()
                                            .px_1()
                                            .h(px(20.0))
                                            .overflow_hidden()
                                            .whitespace_nowrap()
                                            .rounded_sm()
                                            .border_l_2()
                                            .border_color(rgb(event.color))
                                            .text_size(px(10.0))
                                            .cursor_pointer()
                                            .bg(rgb(event_surface(event.color)))
                                            .text_color(rgb(INK))
                                            .child(format!(
                                                "{}{}",
                                                if event.all_day {
                                                    String::new()
                                                } else {
                                                    format!(
                                                        "{} ",
                                                        event
                                                            .start
                                                            .with_timezone(&self.timezone)
                                                            .format("%H:%M")
                                                    )
                                                },
                                                event.title
                                            ))
                                            .on_click(context.listener(
                                                move |app, _, _, context| {
                                                    context.stop_propagation();
                                                    app.show_details(event.clone(), context);
                                                },
                                            ))
                                    })),
                            )
                    }))
            }))
    }
}
