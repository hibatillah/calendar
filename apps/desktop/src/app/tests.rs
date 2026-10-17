use super::*;
use core::prelude::v1::test;
#[gpui::test]
fn activity_scroll_does_not_move_calendar(context: &mut TestAppContext) {
    context.update(gpui_kit::init);
    let (app, context) =
        context.add_window_view(|window, context| CalendarApp::new(window, context, false));
    app.update(context, |app, _| {
        app.activity_open = true;
        app.backend_logs = (0..100)
            .map(|index| calendar_telemetry::LogEntry {
                timestamp: format!("2026-10-15T10:00:{:02}Z", index % 60),
                component: "backend".into(),
                level: "INFO".into(),
                action: "test_activity".into(),
                details: Default::default(),
            })
            .collect();
    });
    context.draw(
        point(px(0.0), px(0.0)),
        size(px(1000.0), px(600.0)),
        |_, _| app.clone().into_any_element(),
    );
    let position = context.debug_bounds("activity-entries").unwrap().center();
    let before = app.read_with(context, |app, _| {
        (app.scroll.offset(), app.date, app.horizontal_fraction)
    });
    context.simulate_mouse_move(position, None, Modifiers::default());
    context.simulate_event(ScrollWheelEvent {
        position,
        delta: ScrollDelta::Pixels(point(px(0.0), px(-80.0))),
        ..Default::default()
    });
    app.read_with(context, |app, _| {
        assert!(app.activity_scroll.offset().y < px(0.0));
        assert_eq!(
            (app.scroll.offset(), app.date, app.horizontal_fraction),
            before
        );
    });
    for delta in [
        point(px(-80.0), px(0.0)),
        point(px(0.0), px(100000.0)),
        point(px(0.0), px(80.0)),
    ] {
        context.simulate_event(ScrollWheelEvent {
            position,
            delta: ScrollDelta::Pixels(delta),
            ..Default::default()
        });
    }
    app.read_with(context, |app, _| {
        assert_eq!(
            (app.scroll.offset(), app.date, app.horizontal_fraction),
            before
        )
    });
}

#[gpui::test]
fn dropped_event_stays_optimistic_until_success_or_failure(context: &mut TestAppContext) {
    context.update(gpui_kit::init);
    let handle = context.add_window(|window, context| CalendarApp::new(window, context, false));
    let (sender, requests) = std::sync::mpsc::channel();
    let (updates, receiver) = std::sync::mpsc::channel();
    handle
        .update(context, |app, window, context| {
            app.connection = service::Connection { sender, receiver };
            app.online = true;
            let mut original = app.new_event(app.date, 540);
            original.revision = 1;
            app.events.push(original.clone());
            app.begin_gesture(
                original.clone(),
                GestureKind::Move,
                point(px(400.0), px(300.0)),
                context,
            );
            let day_width =
                (f32::from(window.viewport_size().width) - app.sidebar_width() - GUTTER) / 7.0;
            app.move_gesture(point(px(400.0 + day_width), px(300.0)), window, context);
            app.finish_gesture(window, context);
            assert!(app.pending);
            assert_eq!(app.events[0].start, original.start + Duration::days(1));
            assert!(matches!(requests.try_recv().unwrap(), Request::Save(_)));
            updates
                .send(Update::Snapshot(vec![original.clone()], true))
                .unwrap();
            app.poll(context);
            assert_eq!(app.events[0].start, original.start + Duration::days(1));
            let mut saved = app.events[0].clone();
            saved.revision = 2;
            updates.send(Update::Saved(saved.clone())).unwrap();
            app.poll(context);
            assert!(!app.pending);
            assert_eq!(app.events[0].start, saved.start);
            assert_eq!(app.events[0].revision, 2);
            let mut moved = saved.clone();
            moved.start += Duration::days(1);
            moved.end += Duration::days(1);
            app.commit(moved.clone(), context);
            assert_eq!(app.events[0].start, moved.start);
            updates
                .send(Update::Failed("test rejection".into()))
                .unwrap();
            app.poll(context);
            assert_eq!(app.events[0].start, saved.start);
            assert!(!app.pending);
            assert!(app.message.contains("failed"));
            drop(requests);
            app.commit(moved, context);
            assert_eq!(app.events[0].start, saved.start);
            assert!(!app.pending);
        })
        .unwrap();
}

#[gpui::test]
fn today_and_month_grid_follow_week_start_preference(context: &mut TestAppContext) {
    context.update(gpui_kit::init);
    let handle = context.add_window(|window, context| CalendarApp::new(window, context, false));
    handle
        .update(context, |app, _, _| {
            let thursday = NaiveDate::from_ymd_opt(2026, 10, 15).unwrap();
            assert_eq!(week_start(thursday, false).day(), 12);
            assert_eq!(week_start(thursday, true).day(), 11);
            app.preferences.sunday_first = false;
            app.go_today();
            assert_eq!(app.date.weekday(), chrono::Weekday::Mon);
            app.preferences.sunday_first = true;
            app.go_today();
            assert_eq!(app.date.weekday(), chrono::Weekday::Sun);
            assert_eq!(app.display_month_cells()[0].weekday(), chrono::Weekday::Sun);
        })
        .unwrap();
}

#[gpui::test]
fn settings_and_event_popover_do_not_scroll_the_timeline(context: &mut TestAppContext) {
    context.update(gpui_kit::init);
    let (app, context) =
        context.add_window_view(|window, context| CalendarApp::new(window, context, false));
    for settings in [false, true] {
        app.update(context, |app, context| {
            let mut event = app.new_event(app.date, 540);
            event.revision = 0;
            app.selected = Some(event);
            app.settings_open = settings;
            context.notify();
        });
        context.draw(
            point(px(0.0), px(0.0)),
            size(px(1050.0), px(680.0)),
            |_, _| app.clone().into_any_element(),
        );
        let before = app.read_with(context, |app, _| (app.date, app.scroll.offset()));
        context.simulate_mouse_move(point(px(800.0), px(350.0)), None, Modifiers::default());
        context.simulate_event(ScrollWheelEvent {
            position: point(px(800.0), px(350.0)),
            delta: ScrollDelta::Pixels(point(px(-70.0), px(-70.0))),
            ..Default::default()
        });
        app.read_with(context, |app, _| {
            assert_eq!(app.date, before.0);
            assert_eq!(app.scroll.offset(), before.1);
        });
    }
}

#[gpui::test]
fn sidebar_buttons_close_and_reopen_with_pointer_clicks(context: &mut TestAppContext) {
    context.update(gpui_kit::init);
    let (app, context) =
        context.add_window_view(|window, context| CalendarApp::new(window, context, false));
    for _ in 0..2 {
        context.draw(
            point(px(0.0), px(0.0)),
            size(px(1000.0), px(600.0)),
            |_, _| app.clone().into_any_element(),
        );
        context.simulate_click(point(px(57.0), px(27.0)), Modifiers::default());
        app.read_with(context, |app, _| assert!(!app.sidebar_open));
        context.draw(
            point(px(0.0), px(0.0)),
            size(px(1000.0), px(600.0)),
            |_, _| app.clone().into_any_element(),
        );
        context.simulate_click(point(px(27.0), px(21.0)), Modifiers::default());
        app.read_with(context, |app, _| assert!(app.sidebar_open));
    }
}

#[gpui::test]
fn view_shortcuts_and_month_navigation(context: &mut TestAppContext) {
    context.update(gpui_kit::init);
    let (app, context) =
        context.add_window_view(|window, context| CalendarApp::new(window, context, false));
    context.draw(
        point(px(0.0), px(0.0)),
        size(px(1000.0), px(600.0)),
        |_, _| app.clone().into_any_element(),
    );
    context.simulate_keystrokes("m");
    app.update(context, |app, context| {
        assert!(app.month_view);
        app.date = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        app.navigate(7, context);
        assert_eq!(app.date, NaiveDate::from_ymd_opt(2027, 1, 1).unwrap());
        app.navigate(-7, context);
        assert_eq!(app.date, NaiveDate::from_ymd_opt(2026, 12, 1).unwrap());
        assert!(app.date_visible(NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()));
        assert!(!app.date_visible(NaiveDate::from_ymd_opt(2027, 1, 1).unwrap()));
        app.toggle_sidebar(context);
        assert_eq!(app.sidebar_width(), 0.0);
    });
    context.draw(
        point(px(0.0), px(0.0)),
        size(px(1000.0), px(600.0)),
        |_, _| app.clone().into_any_element(),
    );
    context.simulate_keystrokes("w");
    app.update(context, |app, _| {
        assert!(!app.month_view);
        app.selected = Some(app.new_event(app.date, 540));
    });
    context.simulate_keystrokes("m");
    app.read_with(context, |app, _| assert!(!app.month_view));
}

#[gpui::test]
fn sidebar_highlights_seven_dates_without_counting_clipped_edges(context: &mut TestAppContext) {
    context.update(gpui_kit::init);
    let handle = context.add_window(|window, context| CalendarApp::new(window, context, false));
    handle
        .update(context, |app, _, _| {
            app.date = NaiveDate::from_ymd_opt(2026, 10, 12).unwrap();
            for fraction in [0.0, f32::EPSILON, 0.01, 0.49, 0.5, 0.99] {
                app.horizontal_fraction = fraction;
                let highlighted: Vec<_> = (0..9)
                    .map(|day| app.date + Duration::days(day))
                    .filter(|date| app.date_visible(*date))
                    .collect();
                let first_day = if fraction < 0.5 { 12 } else { 13 };
                assert_eq!(highlighted.len(), 7);
                assert_eq!(highlighted[0].day(), first_day);
                assert_eq!(highlighted[6].day(), first_day + 6);
            }
        })
        .unwrap();
}

#[gpui::test]
fn horizontal_scroll_tracks_visible_dates_across_year_boundary(context: &mut TestAppContext) {
    context.update(gpui_kit::init);
    let handle = context.add_window(|window, context| CalendarApp::new(window, context, false));
    handle
        .update(context, |app, window, context| {
            app.date = NaiveDate::from_ymd_opt(2026, 12, 28).unwrap();
            let original_vertical = app.scroll.offset().y;
            app.scroll_dates(-150.0, 100.0, context);
            assert_eq!(app.date, NaiveDate::from_ymd_opt(2026, 12, 29).unwrap());
            assert_eq!(app.horizontal_fraction, 0.5);
            assert!(app.date_visible(NaiveDate::from_ymd_opt(2027, 1, 5).unwrap()));
            assert!(!app.date_visible(NaiveDate::from_ymd_opt(2026, 12, 28).unwrap()));
            app.scroll_dates(50.0, 100.0, context);
            assert_eq!(app.horizontal_fraction, 0.0);
            assert!(!app.date_visible(NaiveDate::from_ymd_opt(2027, 1, 5).unwrap()));
            assert_eq!(app.scroll.offset().y, original_vertical);
            app.scroll_dates(200.0, 100.0, context);
            assert_eq!(app.date, NaiveDate::from_ymd_opt(2026, 12, 27).unwrap());
            let event = app.new_event(app.date + Duration::days(2), 540);
            assert_eq!(
                event.start.with_timezone(&app.timezone).date_naive(),
                NaiveDate::from_ymd_opt(2026, 12, 29).unwrap()
            );
            app.scroll_timeline(
                &ScrollWheelEvent {
                    delta: ScrollDelta::Pixels(point(px(0.0), px(-60.0))),
                    ..Default::default()
                },
                window,
                context,
            );
            assert_eq!(app.horizontal_fraction, 0.0);
            app.scroll_timeline(
                &ScrollWheelEvent {
                    delta: ScrollDelta::Pixels(point(px(0.0), px(-60.0))),
                    modifiers: Modifiers {
                        shift: true,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                window,
                context,
            );
            assert!(app.horizontal_fraction > 0.0);
            assert_eq!(app.scroll.offset().y, original_vertical);
        })
        .unwrap();
}

#[gpui::test]
fn timeline_dispatch_keeps_horizontal_and_vertical_scroll_independent(
    context: &mut TestAppContext,
) {
    context.update(gpui_kit::init);
    let (app, context) =
        context.add_window_view(|window, context| CalendarApp::new(window, context, false));
    context.draw(
        point(px(0.0), px(0.0)),
        size(px(1000.0), px(600.0)),
        |_, _| app.clone().into_any_element(),
    );
    let before = app.read_with(context, |app, _| (app.date, app.scroll.offset().y));
    context.simulate_mouse_move(point(px(400.0), px(300.0)), None, Modifiers::default());
    context.simulate_event(ScrollWheelEvent {
        position: point(px(400.0), px(300.0)),
        delta: ScrollDelta::Pixels(point(px(-50.0), px(0.0))),
        ..Default::default()
    });
    app.read_with(context, |app, _| {
        assert!(app.horizontal_fraction > 0.0 || app.date > before.0);
        assert_eq!(app.scroll.offset().y, before.1);
    });
    let horizontal_before = app.read_with(context, |app, _| (app.date, app.horizontal_fraction));
    context.simulate_event(ScrollWheelEvent {
        position: point(px(400.0), px(300.0)),
        delta: ScrollDelta::Pixels(point(px(0.0), px(50.0))),
        ..Default::default()
    });
    app.read_with(context, |app, _| {
        assert_eq!((app.date, app.horizontal_fraction), horizontal_before);
        assert!(
            app.scroll.offset().y > before.1,
            "before={:?} after={:?}",
            before.1,
            app.scroll.offset().y
        );
    });
}

#[gpui::test]
fn offline_edits_leave_events_unchanged(context: &mut TestAppContext) {
    context.update(gpui_kit::init);
    let handle = context.add_window(|window, context| CalendarApp::new(window, context, false));
    handle
        .update(context, |app, _, context| {
            let event = app.new_event(app.date, 540);
            app.commit(event, context);
            assert!(!app.pending);
            assert!(app.events.is_empty());
            assert!(app.message.contains("Offline"));
        })
        .unwrap();
}

#[gpui::test]
fn move_and_resize_preserve_valid_ranges(context: &mut TestAppContext) {
    context.update(gpui_kit::init);
    let handle = context.add_window(|window, context| CalendarApp::new(window, context, false));
    handle
        .update(context, |app, window, context| {
            app.online = true;
            let event = app.new_event(app.date, 540);
            app.events.push(event.clone());
            app.begin_gesture(
                event.clone(),
                GestureKind::Move,
                point(px(300.0), px(300.0)),
                context,
            );
            app.move_gesture(point(px(300.0), px(300.0 + HOUR)), window, context);
            assert_eq!(app.events[0].start, event.start + Duration::hours(1));
            assert_eq!(app.events[0].end - app.events[0].start, Duration::hours(1));
            app.begin_gesture(
                event.clone(),
                GestureKind::End,
                point(px(300.0), px(300.0)),
                context,
            );
            app.move_gesture(point(px(300.0), px(100.0)), window, context);
            assert_eq!(
                app.events[0].end - app.events[0].start,
                Duration::minutes(15)
            );
        })
        .unwrap();
}
