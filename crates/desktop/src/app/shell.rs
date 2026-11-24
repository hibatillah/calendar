use super::*;

impl Render for CalendarApp {
    fn render(&mut self, window: &mut Window, context: &mut Context<Self>) -> impl IntoElement {
        let mut root = div()
            .id("calendar-app")
            .track_focus(&self.focus)
            .relative()
            .size_full()
            .flex()
            .text_color(rgb(INK))
            .text_size(px(13.0))
            .bg(rgb(0x191919))
            .font_family("Inter")
            .on_mouse_move(
                context.listener(|app, event: &MouseMoveEvent, window, context| {
                    app.move_gesture(event.position, window, context)
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                context.listener(|app, _, window, context| app.finish_gesture(window, context)),
            )
            .on_key_down(
                context.listener(|app, event: &KeyDownEvent, window, context| {
                    if app.selected.is_none()
                        && app.gesture.is_none()
                        && !app.activity_open
                        && !app.settings_open
                        && !app.profile_open
                        && !event.keystroke.modifiers.control
                        && !event.keystroke.modifiers.alt
                        && !event.keystroke.modifiers.platform
                    {
                        match event.keystroke.key.as_str() {
                            "w" => app.set_month_view(false, context),
                            "m" => app.set_month_view(true, context),
                            "t" => {
                                app.go_today();
                                context.notify();
                            }
                            _ => (),
                        }
                    }
                    if event.keystroke.key == "escape" {
                        app.view_menu_open = false;
                        app.settings_open = false;
                        app.profile_open = false;
                        if let Some(gesture) = app.gesture.take() {
                            app.events
                                .retain(|event| event.event_id != gesture.event.event_id);
                            if gesture.event.revision >= 0 {
                                app.events.push(gesture.event);
                            }
                        }
                        app.selected = None;
                        if app.agenda_only {
                            window.remove_window();
                        } else {
                            app.focus.focus(window, context);
                            context.notify();
                        }
                    }
                }),
            );
        if self.agenda_only {
            return root.child(self.agenda(context)).into_any_element();
        }
        let first_date = self.date;
        let day_width = ((f32::from(window.viewport_size().width) - self.sidebar_width() - GUTTER)
            / 7.0)
            .max(80.0);
        root = root
            .when(self.sidebar_open, |root| root.child(self.sidebar(context)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .flex_col()
                    .bg(rgb(0x191919))
                    .child(
                        TitleBar::new()
                            .h(px(42.0))
                            .border_b_0()
                            .bg(rgb(0x191919))
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .justify_end()
                                    .items_center()
                                    .gap_2()
                                    .pr_4()
                                    .when(!self.sidebar_open, |element| {
                                        element.child(
                                            icon_button("sidebar-open", "icons/panel-left.svg")
                                                .on_click(context.listener(
                                                    |app, _, _, context| {
                                                        app.toggle_sidebar(context)
                                                    },
                                                )),
                                        )
                                    })
                                    .child(
                                        div()
                                            .flex_1()
                                            .h_full()
                                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                                window.start_window_move()
                                            }),
                                    )
                                    .child(
                                        div()
                                            .id("view-selector")
                                            .occlude()
                                            .on_mouse_down(MouseButton::Left, |_, _, context| {
                                                context.stop_propagation()
                                            })
                                            .cursor_pointer()
                                            .on_click(context.listener(|app, _, _, context| {
                                                app.view_menu_open = !app.view_menu_open;
                                                context.notify();
                                            }))
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .px_3()
                                            .py_1()
                                            .rounded_md()
                                            .bg(rgb(0x272727))
                                            .border_1()
                                            .border_color(rgb(0x343434))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(if self.month_view { "Month" } else { "Week" })
                                            .child(
                                                svg()
                                                    .path("icons/chevron-down.svg")
                                                    .size(px(16.0))
                                                    .text_color(rgb(0xbdbdbd)),
                                            ),
                                    )
                                    .child(
                                        button("today", "Today")
                                            .px_3()
                                            .bg(rgb(0x272727))
                                            .border_1()
                                            .border_color(rgb(0x343434))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .on_click(context.listener(|app, _, _, context| {
                                                app.go_today();
                                                app.horizontal_fraction = 0.0;
                                                context.notify();
                                            })),
                                    )
                                    .child(
                                        icon_button(
                                            "previous",
                                            if self.month_view {
                                                "icons/chevron-up.svg"
                                            } else {
                                                "icons/chevron-left.svg"
                                            },
                                        )
                                        .text_size(px(21.0))
                                        .text_color(rgb(MUTED))
                                        .on_click(
                                            context.listener(|app, _, _, context| {
                                                app.navigate(-7, context)
                                            }),
                                        ),
                                    )
                                    .child(
                                        icon_button(
                                            "next",
                                            if self.month_view {
                                                "icons/chevron-down.svg"
                                            } else {
                                                "icons/chevron-right.svg"
                                            },
                                        )
                                        .text_size(px(21.0))
                                        .text_color(rgb(MUTED))
                                        .on_click(
                                            context.listener(|app, _, _, context| {
                                                app.navigate(7, context)
                                            }),
                                        ),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .h(px(40.0))
                            .flex_shrink_0()
                            .px_5()
                            .flex()
                            .items_end()
                            .pb_1()
                            .child(
                                div()
                                    .text_size(px(22.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0xd4d4d4))
                                    .child(self.date.format("%B %Y").to_string()),
                            ),
                    )
                    .when(!self.month_view, |element| {
                        element
                            .child(
                                div()
                                    .h(px(26.0))
                                    .flex_shrink_0()
                                    .flex()
                                    .items_center()
                                    .border_b_1()
                                    .border_color(rgb(0xffffff).opacity(0.07))
                                    .child(
                                        div()
                                            .w(px(GUTTER))
                                            .flex_shrink_0()
                                            .text_center()
                                            .text_size(px(9.0))
                                            .text_color(rgb(MUTED))
                                            .child("GMT+7"),
                                    )
                                    .child(
                                        div()
                                            .relative()
                                            .flex_1()
                                            .h_full()
                                            .overflow_hidden()
                                            .on_scroll_wheel(
                                                context.listener(Self::scroll_timeline),
                                            )
                                            .children((0..8).map(|day| {
                                                let date = first_date + Duration::days(day);
                                                let today = date
                                                    == Utc::now()
                                                        .with_timezone(&self.timezone)
                                                        .date_naive();
                                                div()
                                                    .absolute()
                                                    .top_0()
                                                    .left(px((day as f32
                                                        - self.horizontal_fraction)
                                                        * day_width))
                                                    .h_full()
                                                    .w(px(day_width))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .gap_2()
                                                    .text_color(rgb(if today {
                                                        INK
                                                    } else {
                                                        MUTED
                                                    }))
                                                    .child(date.format("%a").to_string())
                                                    .child(
                                                        div()
                                                            .w(px(20.0))
                                                            .h(px(22.0))
                                                            .rounded_md()
                                                            .flex()
                                                            .items_center()
                                                            .justify_center()
                                                            .when(today, |element| {
                                                                element
                                                                    .bg(rgb(0xf45151))
                                                                    .text_color(rgb(0xffffff))
                                                            })
                                                            .child(date.day().to_string()),
                                                    )
                                            })),
                                    ),
                            )
                            .child(
                                div()
                                    .h(px(24.0))
                                    .flex_shrink_0()
                                    .flex()
                                    .border_b_1()
                                    .border_color(rgb(0xffffff).opacity(0.07))
                                    .child(
                                        div()
                                            .w(px(GUTTER))
                                            .flex_shrink_0()
                                            .text_size(px(9.0))
                                            .text_color(rgb(MUTED))
                                            .pt_2()
                                            .text_center()
                                            .child("all-day"),
                                    )
                                    .child(
                                        div()
                                            .relative()
                                            .flex_1()
                                            .h_full()
                                            .overflow_hidden()
                                            .on_scroll_wheel(
                                                context.listener(Self::scroll_timeline),
                                            )
                                            .children((0..8).map(|day| {
                                                let date = first_date + Duration::days(day);
                                                div()
                                                    .absolute()
                                                    .top_0()
                                                    .left(px((day as f32
                                                        - self.horizontal_fraction)
                                                        * day_width))
                                                    .h_full()
                                                    .w(px(day_width))
                                                    .overflow_hidden()
                                                    .children(
                                                        self.events
                                                            .iter()
                                                            .filter(|event| {
                                                                event.all_day
                                                                    && event
                                                                        .start
                                                                        .with_timezone(
                                                                            &self.timezone,
                                                                        )
                                                                        .date_naive()
                                                                        <= date
                                                                    && event
                                                                        .end
                                                                        .with_timezone(
                                                                            &self.timezone,
                                                                        )
                                                                        .date_naive()
                                                                        > date
                                                                    && self.calendar_visible(event)
                                                            })
                                                            .map(|event| {
                                                                let event = event.clone();
                                                                div()
                                                        .id(SharedString::from(format!(
                                                            "all-day-{}-{day}",
                                                            event.event_id
                                                        )))
                                                        .m_1()
                                                        .px_2()
                                                        .rounded_md()
                                                        .text_size(px(10.0))
                                                        .bg(rgb(event.color).opacity(0.2))
                                                        .cursor_pointer()
                                                        .child(event.title.clone())
                                                        .on_click(context.listener(
                                                            move |app, _, window, context| {
                                                                app.edit(
                                                                    event.clone(),
                                                                    window,
                                                                    context,
                                                                )
                                                            },
                                                        ))
                                                            }),
                                                    )
                                            })),
                                    ),
                            )
                            .child(self.week_grid(window, context))
                    })
                    .when(self.month_view, |element| {
                        element.child(self.month_grid(context))
                    })
                    .when(
                        self.pending
                            || self.message.contains("not configured")
                            || self.message.contains("failed")
                            || self.message.contains("unavailable"),
                        |element| {
                            element.child(
                                div()
                                    .absolute()
                                    .bottom_3()
                                    .right_3()
                                    .max_w(px(420.0))
                                    .p_3()
                                    .rounded_md()
                                    .bg(rgb(0x303030))
                                    .text_size(px(11.0))
                                    .text_color(rgb(INK))
                                    .child(self.message.clone()),
                            )
                        },
                    ),
            );
        if self.view_menu_open {
            root = root.child(
                div()
                    .absolute()
                    .top(px(42.0))
                    .right(px(230.0))
                    .w(px(150.0))
                    .p_1()
                    .rounded_md()
                    .bg(rgb(0x292929))
                    .border_1()
                    .border_color(rgb(0x404040))
                    .child(button("view-week", "Week                  W").on_click(
                        context.listener(|app, _, _, context| app.set_month_view(false, context)),
                    ))
                    .child(button("view-month", "Month                M").on_click(
                        context.listener(|app, _, _, context| app.set_month_view(true, context)),
                    )),
            );
        }
        if !self.agenda_only {
            let account = self.google_connection.accounts.get(self.active_account);
            let avatar = div()
                .size(px(28.0))
                .rounded_full()
                .overflow_hidden()
                .bg(rgb(0x3c526a))
                .when_some(
                    account.filter(|account| !account.picture.is_empty()),
                    |element, account| {
                        element.child(
                            img(std::path::PathBuf::from(account.picture.clone()))
                                .size_full()
                                .object_fit(ObjectFit::Cover),
                        )
                    },
                )
                .when(
                    account.is_none_or(|account| account.picture.is_empty()),
                    |element| {
                        element.child(
                            div()
                                .size_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    account
                                        .and_then(|account| {
                                            account
                                                .name
                                                .chars()
                                                .next()
                                                .or_else(|| account.email.chars().next())
                                        })
                                        .unwrap_or('G')
                                        .to_string(),
                                ),
                        )
                    },
                );
            root = root.child(
                div()
                    .id("profile-toggle")
                    .absolute()
                    .left(px(
                        self.sidebar_width() + if self.sidebar_open { 18.0 } else { 56.0 }
                    ))
                    .top(px(5.0))
                    .size(px(30.0))
                    .occlude()
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, |_, _, context| {
                        context.stop_propagation()
                    })
                    .on_click(context.listener(|app, _, _, context| {
                        app.profile_open = !app.profile_open;
                        context.notify();
                    }))
                    .child(avatar),
            );
        }
        if self.profile_open {
            root = root.child(self.profile_menu(context));
        }
        if self.settings_open {
            root = root.child(self.settings(context));
        }
        if self.selected.is_some() && !self.settings_open {
            root = root.child(self.editor(context));
        }
        if self.activity_open && !self.settings_open {
            root = root.child(self.activity_panel(context));
        }
        root.into_any_element()
    }
}
