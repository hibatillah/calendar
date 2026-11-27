use super::*;
use crate::service::{AccountView, Update, snapshot::CalendarView};
use core::prelude::v1::test;

pub(super) fn new_app(window: &mut Window, context: &mut Context<CalendarApp>) -> CalendarApp {
    let model = context.new(|_| Model::default());
    CalendarApp::new(
        window,
        context,
        false,
        model,
        Handle::disconnected(),
        Preferences::default(),
    )
}

pub(super) fn event_at(app: &CalendarApp, date: NaiveDate, minute: u32) -> CalendarEvent {
    let start = calendar_core::resolve_local(date, minute, app.timezone).unwrap();
    CalendarEvent {
        event_id: "account/work/one".into(),
        calendar_id: "account/work".into(),
        title: "Planning".into(),
        start,
        end: start + Duration::hours(1),
        timezone: app.timezone.name().into(),
        all_day: false,
        description: String::new(),
        location: String::new(),
        color: 0x8c9ef8,
        revision: 0,
        read_only: false,
    }
}

fn account(error: Option<&str>) -> AccountView {
    AccountView {
        account_id: "account".into(),
        email: "a@example.com".into(),
        name: "A".into(),
        permissions: None,
        error: error.map(str::to_owned),
        calendars: vec![CalendarView {
            key: "account/work".into(),
            summary: "Work".into(),
            color: 0x8c9ef8,
            writable: true,
        }],
    }
}
#[gpui::test]
fn today_and_month_grid_follow_week_start_preference(context: &mut TestAppContext) {
    context.update(gpui_kit::init);
    let handle = context.add_window(new_app);
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
    let (app, context) = context.add_window_view(new_app);
    for settings in [false, true] {
        app.update(context, |app, context| {
            app.selected = Some(event_at(app, app.date, 540));
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
    let (app, context) = context.add_window_view(new_app);
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
    let (app, context) = context.add_window_view(new_app);
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
        app.selected = Some(event_at(app, app.date, 540));
    });
    context.simulate_keystrokes("m");
    app.read_with(context, |app, _| assert!(!app.month_view));
}

#[gpui::test]
fn sidebar_highlights_seven_dates_without_counting_clipped_edges(context: &mut TestAppContext) {
    context.update(gpui_kit::init);
    let handle = context.add_window(new_app);
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
    let handle = context.add_window(new_app);
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
    let (app, context) = context.add_window_view(new_app);
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
fn cached_events_show_first_and_a_failed_refresh_keeps_them_and_says_why(
    context: &mut TestAppContext,
) {
    context.update(gpui_kit::init);
    let model = context.new(|_| Model::default());
    let feed = model.clone();
    let (app, context) = context.add_window_view(|window, context| {
        CalendarApp::new(
            window,
            context,
            false,
            model,
            Handle::disconnected(),
            Preferences::default(),
        )
    });
    let (events, snapshot) = app.read_with(context, |app, _| {
        let today = Utc::now().with_timezone(&app.timezone).date_naive();
        let events = vec![event_at(app, today, 600)];
        let snapshot = Snapshot {
            configured: true,
            accounts: vec![account(None)],
            events: events.clone(),
            covered: Some((
                Utc::now() - Duration::days(30),
                Utc::now() + Duration::days(180),
            )),
            ..Snapshot::default()
        };
        (events, snapshot)
    });

    feed.update(context, |model, context| {
        model.apply(Update::Snapshot(snapshot.clone()));
        context.notify();
    });
    app.read_with(context, |app, _| {
        assert_eq!(app.snapshot.events, events);
        assert!(app.notices().is_empty());
    });

    let mut failed = snapshot;
    failed.accounts = vec![account(Some("Google is unreachable. Try again online."))];
    feed.update(context, |model, context| {
        model.apply(Update::Snapshot(failed));
        context.notify();
    });
    app.read_with(context, |app, _| {
        assert_eq!(app.snapshot.events, events, "saved events stay visible");
        let notices = app.notices();
        assert_eq!(notices.len(), 1);
        assert!(notices[0].contains("a@example.com") && notices[0].contains("unreachable"));
    });
}

#[gpui::test]
fn scrolling_outside_the_cached_window_says_so_instead_of_looking_empty(
    context: &mut TestAppContext,
) {
    context.update(gpui_kit::init);
    let model = context.new(|_| Model::default());
    let feed = model.clone();
    let (app, context) = context.add_window_view(|window, context| {
        CalendarApp::new(
            window,
            context,
            false,
            model,
            Handle::disconnected(),
            Preferences::default(),
        )
    });
    feed.update(context, |model, context| {
        model.apply(Update::Snapshot(Snapshot {
            configured: true,
            accounts: vec![account(None)],
            events: Vec::new(),
            covered: Some((
                Utc::now() - Duration::days(30),
                Utc::now() + Duration::days(180),
            )),
            ..Snapshot::default()
        }));
        context.notify();
    });
    app.update(context, |app, _| {
        app.go_today();
        assert!(app.notices().is_empty());
        app.date += Duration::days(400);
        let notices = app.notices();
        assert_eq!(notices.len(), 1);
        assert!(notices[0].contains("not free"));
        app.date -= Duration::days(800);
        assert_eq!(app.notices().len(), 1);
        app.month_view = true;
        app.date = Utc::now().with_timezone(&app.timezone).date_naive() + Duration::days(60);
        assert!(app.notices().is_empty());
    });
}

#[gpui::test]
fn empty_states_distinguish_unconfigured_builds_from_missing_accounts(
    context: &mut TestAppContext,
) {
    context.update(gpui_kit::init);
    let model = context.new(|_| Model::default());
    let feed = model.clone();
    let (app, context) = context.add_window_view(|window, context| {
        CalendarApp::new(
            window,
            context,
            false,
            model,
            Handle::disconnected(),
            Preferences::default(),
        )
    });
    app.read_with(context, |app, _| {
        assert_eq!(app.notices(), ["Sign-in is not configured in this build."]);
    });
    feed.update(context, |model, context| {
        model.apply(Update::Snapshot(Snapshot {
            configured: true,
            ..Snapshot::default()
        }));
        context.notify();
    });
    app.read_with(context, |app, _| {
        assert!(app.notices()[0].starts_with("Connect a Google account"));
    });
    feed.update(context, |model, context| {
        model.apply(Update::Error(
            "The local calendar cache is unavailable.".into(),
        ));
        context.notify();
    });
    app.read_with(context, |app, _| {
        assert!(app.notices()[0].contains("cache is unavailable"));
    });
}

#[gpui::test]
fn details_open_read_only_and_escape_closes_them(context: &mut TestAppContext) {
    context.update(gpui_kit::init);
    let (app, context) = context.add_window_view(new_app);
    app.update(context, |app, context| {
        let event = event_at(app, app.date, 540);
        app.open_event(event, context);
        assert!(app.selected.is_some());
    });
    context.draw(
        point(px(0.0), px(0.0)),
        size(px(1050.0), px(680.0)),
        |_, _| app.clone().into_any_element(),
    );
    context.simulate_keystrokes("escape");
    app.read_with(context, |app, _| assert!(app.selected.is_none()));
}

#[gpui::test]
fn changing_week_start_realigns_the_view_and_persists(context: &mut TestAppContext) {
    context.update(gpui_kit::init);
    let path = std::env::temp_dir()
        .join(format!("calendar-week-start-{}", uuid::Uuid::new_v4()))
        .join("preferences.json");
    let saved = path.clone();
    let (app, context) = context.add_window_view(move |window, context| {
        let model = context.new(|_| Model::default());
        let preferences = Preferences::load(Some(saved));
        CalendarApp::new(
            window,
            context,
            false,
            model,
            Handle::disconnected(),
            preferences,
        )
    });
    app.update(context, |app, context| {
        app.date = NaiveDate::from_ymd_opt(2026, 10, 15).unwrap();
        app.set_week_start(true, context);
        assert_eq!(app.date, NaiveDate::from_ymd_opt(2026, 10, 11).unwrap());
        app.set_week_start(false, context);
        assert_eq!(app.date, NaiveDate::from_ymd_opt(2026, 10, 5).unwrap());
        app.set_week_start(true, context);
        app.set_tray_all_day(false, context);
    });
    let reloaded = Preferences::load(Some(path.clone()));
    assert!(reloaded.sunday_first && !reloaded.tray_all_day);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[gpui::test]
fn views_use_the_zone_the_snapshot_carries_and_keep_it_when_none_is_recorded(
    context: &mut TestAppContext,
) {
    use chrono::TimeZone;

    context.update(gpui_kit::init);
    let model = context.new(|_| Model::default());
    let feed = model.clone();
    let (app, context) = context.add_window_view(|window, context| {
        CalendarApp::new(
            window,
            context,
            false,
            model,
            Handle::disconnected(),
            Preferences::default(),
        )
    });
    let send = |context: &mut VisualTestContext, zone| {
        feed.update(context, |model, context| {
            model.apply(Update::Snapshot(Snapshot {
                zone,
                ..Snapshot::default()
            }));
            context.notify();
        });
    };

    send(context, Some(chrono_tz::Pacific::Auckland));
    app.read_with(context, |app, _| {
        assert_eq!(app.timezone, chrono_tz::Pacific::Auckland);
        // 11:00 UTC on the 14th is already the 15th in Auckland (UTC+13 in October).
        let instant = Utc.with_ymd_and_hms(2026, 10, 14, 11, 0, 0).unwrap();
        let date = instant.with_timezone(&app.timezone).date_naive();
        assert_eq!(date, NaiveDate::from_ymd_opt(2026, 10, 15).unwrap());
    });

    send(context, Some(chrono_tz::America::New_York));
    app.read_with(context, |app, _| {
        assert_eq!(app.timezone, chrono_tz::America::New_York);
    });
    send(context, None);
    app.read_with(context, |app, _| {
        assert_eq!(app.timezone, chrono_tz::America::New_York);
    });
}
