use super::tests::new_app;
use super::*;
use core::prelude::v1::test;

fn temporary_preferences() -> (std::path::PathBuf, Preferences) {
    let path = std::env::temp_dir()
        .join(format!("calendar-working-hours-{}", uuid::Uuid::new_v4()))
        .join("preferences.json");
    let preferences = Preferences::load(Some(path.clone()));
    (path, preferences)
}

fn set_text(
    input: &Entity<gpui_kit::component::input::InputState>,
    text: &str,
    window: &mut Window,
    context: &mut Context<CalendarApp>,
) {
    input.update(context, |state, context| {
        state.set_value(text.to_owned(), window, context)
    });
}

#[gpui::test]
fn editing_a_day_persists_and_shades_the_expected_ranges(context: &mut TestAppContext) {
    context.update(gpui_kit::init);
    let (app, context) = context.add_window_view(new_app);
    let (path, preferences) = temporary_preferences();
    app.update_in(context, |app, window, context| {
        app.preferences = preferences;
        app.settings_open = true;
        app.select_settings_section("working-hours", window, context);
        let wednesday = &app.working_form.as_ref().unwrap().days[2];
        let (start, end) = (wednesday.start.clone(), wednesday.end.clone());
        set_text(&start, "10:00", window, context);
        set_text(&end, "16:30", window, context);
        app.save_working_hours(context);

        let day = app.preferences.working_hours.days[2];
        assert_eq!((day.start_minute, day.end_minute), (600, 990));
        let wednesday = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap();
        assert_eq!(
            app.off_ranges(wednesday),
            [Some((0, 600)), Some((990, 1440))]
        );
        assert!(matches!(
            app.working_form.as_ref().unwrap().status,
            Some(Ok(()))
        ));
    });
    context.draw(
        point(px(0.0), px(0.0)),
        size(px(1050.0), px(680.0)),
        |_, _| app.clone().into_any_element(),
    );
    assert_eq!(
        Preferences::load(Some(path.clone())).working_hours,
        app.read_with(context, |app, _| app.preferences.working_hours)
    );
    app.update(context, |app, _| {
        app.close_settings();
        assert!(app.working_form.is_none() && app.settings_section == "general");
    });
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[gpui::test]
fn invalid_hours_are_rejected_and_nothing_changes(context: &mut TestAppContext) {
    context.update(gpui_kit::init);
    let (app, context) = context.add_window_view(new_app);
    let (path, preferences) = temporary_preferences();
    app.update_in(context, |app, window, context| {
        app.preferences = preferences;
        let before = app.preferences.working_hours;
        app.select_settings_section("working-hours", window, context);
        let monday = &app.working_form.as_ref().unwrap().days[0];
        let (start, end) = (monday.start.clone(), monday.end.clone());
        for (start_text, end_text) in [("09:00", "09:00"), ("17:00", "09:00"), ("9am", "17:00")] {
            set_text(&start, start_text, window, context);
            set_text(&end, end_text, window, context);
            app.save_working_hours(context);
            let status = app.working_form.as_ref().unwrap().status.clone();
            assert!(matches!(status, Some(Err(_))), "{start_text}-{end_text}");
            assert_eq!(app.preferences.working_hours, before);
        }
    });
    assert!(!path.exists());
}

#[gpui::test]
fn non_working_days_are_shaded_whole_in_either_week_start(context: &mut TestAppContext) {
    context.update(gpui_kit::init);
    let handle = context.add_window(new_app);
    handle
        .update(context, |app, _, _| {
            let thursday = NaiveDate::from_ymd_opt(2026, 10, 15).unwrap();
            for (sunday_first, whole_day_columns) in [(false, [5, 6]), (true, [0, 6])] {
                let first_date = week_start(thursday, sunday_first);
                for column in 0..7 {
                    let ranges = app.off_ranges(first_date + Duration::days(column));
                    let shaded_whole_day = ranges[0] == Some((0, 1440));
                    assert_eq!(
                        shaded_whole_day,
                        whole_day_columns.contains(&column),
                        "sunday_first {sunday_first}, column {column}"
                    );
                }
            }
            app.preferences.working_hours.enabled = false;
            assert_eq!(app.off_ranges(thursday), [None, None]);
        })
        .unwrap();
}
