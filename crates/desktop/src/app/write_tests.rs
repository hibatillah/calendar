//! Write lifecycle in the views. The handle is disconnected, so nothing reaches Google: results
//! are fed through the model the way the service thread would.

use super::tests::event_at;
use super::*;
use crate::service::{
    AccountView, Update, WriteResult,
    snapshot::{CalendarView, PendingWrite},
};
use calendar_google::Permissions;
use calendar_sync::{OperationKind, WriteError, Written};
use core::prelude::v1::test;
use writes::Action;

fn snapshot(events: &[CalendarEvent], writable: bool, pending: Vec<PendingWrite>) -> Snapshot {
    Snapshot {
        configured: true,
        accounts: vec![AccountView {
            account_id: "account".into(),
            email: "a@example.com".into(),
            name: "A".into(),
            permissions: Some(Permissions {
                calendar_list: true,
                read_events: true,
                write_events: true,
            }),
            error: None,
            calendars: vec![CalendarView {
                key: "account/work".into(),
                summary: "Work".into(),
                color: 0x8c9ef8,
                writable,
            }],
        }],
        events: events.to_vec(),
        covered: Some((
            Utc::now() - Duration::days(30),
            Utc::now() + Duration::days(180),
        )),
        pending,
        ..Snapshot::default()
    }
}

fn send(model: &Entity<Model>, context: &mut VisualTestContext, update: Update) {
    model.update(context, |model, context| {
        model.apply(update);
        context.notify();
    });
}

fn result(operation_id: &str, outcome: Result<Written, WriteError>) -> Update {
    Update::Write(WriteResult {
        operation_id: operation_id.into(),
        outcome,
    })
}

fn world(
    context: &mut TestAppContext,
    writable: bool,
) -> (
    Entity<Model>,
    Entity<CalendarApp>,
    CalendarEvent,
    &mut VisualTestContext,
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
    let event = app.read_with(context, |app, _| event_at(app, app.date, 540));
    send(
        &feed,
        context,
        Update::Snapshot(snapshot(std::slice::from_ref(&event), writable, Vec::new())),
    );

    (feed, app, event, context)
}

fn moved(event: &CalendarEvent) -> CalendarEvent {
    CalendarEvent {
        start: event.start + Duration::minutes(30),
        end: event.end + Duration::minutes(30),
        ..event.clone()
    }
}

fn submit(
    app: &Entity<CalendarApp>,
    context: &mut VisualTestContext,
    action: Action,
) -> Result<String, String> {
    app.update(context, |app, context| app.submit(action, context))
}

#[gpui::test]
fn moved_event_keeps_its_placement_while_saving_and_a_snapshot_does_not_undo_it(
    context: &mut TestAppContext,
) {
    let (feed, app, event, context) = world(context, true);
    let target = moved(&event);
    let operation_id = submit(&app, context, Action::Update(target.clone())).unwrap();
    app.read_with(context, |app, _| {
        assert_eq!(app.events()[0].start, target.start);
        assert!(app.is_saving(&event.event_id));
    });

    // A background sync that still has the old version.
    send(
        &feed,
        context,
        Update::Snapshot(snapshot(std::slice::from_ref(&event), true, Vec::new())),
    );
    app.read_with(context, |app, _| {
        assert_eq!(app.events().len(), 1);
        assert_eq!(app.events()[0].start, target.start);
    });

    let confirmed = CalendarEvent {
        revision: 1,
        ..target
    };
    send(
        &feed,
        context,
        Update::Snapshot(snapshot(std::slice::from_ref(&confirmed), true, Vec::new())),
    );
    send(
        &feed,
        context,
        result(
            &operation_id,
            Ok(Written {
                event_id: "one".into(),
            }),
        ),
    );
    app.read_with(context, |app, _| {
        assert!(app.writes.is_empty());
        assert_eq!(*app.events(), [confirmed]);
        assert!(app.notices().is_empty());
    });
}

#[gpui::test]
fn failed_move_restores_the_position_and_says_why(context: &mut TestAppContext) {
    let (feed, app, event, context) = world(context, true);
    let operation_id = submit(&app, context, Action::Update(moved(&event))).unwrap();
    send(
        &feed,
        context,
        result(&operation_id, Err(WriteError::NotApplied)),
    );
    app.read_with(context, |app, _| {
        assert_eq!(&*app.events(), std::slice::from_ref(&event));
        let notices = app.notices();
        assert_eq!(notices.len(), 1);
        assert!(notices[0].contains("did not apply"));
    });
}

#[gpui::test]
fn unknown_outcome_keeps_the_placement_until_the_pending_marker_clears(
    context: &mut TestAppContext,
) {
    let (feed, app, event, context) = world(context, true);
    let target = moved(&event);
    let operation_id = submit(&app, context, Action::Update(target.clone())).unwrap();
    let pending = vec![PendingWrite {
        operation_id: operation_id.clone(),
        event_id: event.event_id.clone(),
        kind: OperationKind::Update,
    }];
    send(
        &feed,
        context,
        Update::Snapshot(snapshot(std::slice::from_ref(&event), true, pending)),
    );
    send(
        &feed,
        context,
        result(&operation_id, Err(WriteError::Unknown)),
    );
    app.read_with(context, |app, _| {
        assert_eq!(app.events()[0].start, target.start);
        assert!(app.notices()[0].contains("Could not confirm"));
    });

    // Marker gone and the event unchanged: it was not saved.
    send(
        &feed,
        context,
        Update::Snapshot(snapshot(std::slice::from_ref(&event), true, Vec::new())),
    );
    app.read_with(context, |app, _| {
        assert!(app.writes.is_empty());
        assert_eq!(&*app.events(), std::slice::from_ref(&event));
        assert!(app.notices()[0].contains("did not save"));
    });
}

#[gpui::test]
fn read_only_calendar_blocks_writes_before_anything_is_queued(context: &mut TestAppContext) {
    let (_, app, event, context) = world(context, false);
    for action in [Action::Update(moved(&event)), Action::Delete(event.clone())] {
        let reason = submit(&app, context, action).unwrap_err();
        assert!(reason.contains("read-only"));
    }
    app.read_with(context, |app, _| {
        assert!(app.writes.is_empty());
        let draft = app.draft(app.date, 600, 60);
        assert!(draft.unwrap_err().contains("Connect a Google account"));
    });
}

#[gpui::test]
fn dragging_moves_by_snapped_minutes_and_a_plain_click_only_opens_details(
    context: &mut TestAppContext,
) {
    let (_, app, event, context) = world(context, true);
    app.update_in(context, |app, window, context| {
        app.begin_gesture(
            GestureKind::Move,
            event.clone(),
            point(px(100.0), px(100.0)),
            context,
        );
        app.finish_gesture(window, context);
        assert!(app.writes.is_empty());
        assert_eq!(app.selected.as_ref(), Some(&event));

        app.begin_gesture(
            GestureKind::Move,
            event.clone(),
            point(px(100.0), px(100.0)),
            context,
        );
        // 24 px is half an hour at 48 px per hour.
        app.move_gesture(point(px(100.0), px(124.0)), window, context);
        assert_eq!(app.events()[0].start, moved(&event).start);
        app.finish_gesture(window, context);
        assert_eq!(app.writes.len(), 1);
        assert_eq!(
            app.writes[0].placement.as_ref().unwrap().start,
            moved(&event).start
        );
    });
}

#[gpui::test]
fn resizing_changes_only_the_end(context: &mut TestAppContext) {
    let (_, app, event, context) = world(context, true);
    app.update_in(context, |app, window, context| {
        app.begin_gesture(
            GestureKind::Resize,
            event.clone(),
            point(px(100.0), px(100.0)),
            context,
        );
        app.move_gesture(point(px(100.0), px(148.0)), window, context);
        app.finish_gesture(window, context);
        let placement = app.writes[0].placement.clone().unwrap();
        assert_eq!(placement.start, event.start);
        assert_eq!(placement.end, event.end + Duration::hours(1));
    });
}

#[gpui::test]
fn retry_after_not_applied_uses_a_new_operation_and_a_new_draft_id(context: &mut TestAppContext) {
    let (feed, app, _, context) = world(context, true);
    app.update_in(context, |app, window, context| {
        let draft = app.draft(app.date, 600, 60).unwrap();
        app.open_editor(draft, true, window, context);
        assert_eq!(app.events().len(), 2, "the open draft is visible");
    });
    let title = app.read_with(context, |app, _| app.editor.as_ref().unwrap().title.clone());
    title.update_in(context, |input, window, context| {
        input.set_value("Lunch", window, context)
    });

    app.update(context, |app, context| app.save_editor(context));
    let (first_operation, first_key) = app.read_with(context, |app, _| {
        let editor = app.editor.as_ref().unwrap();
        assert_eq!(
            editor.in_flight.as_deref(),
            Some(&*app.writes[0].operation_id)
        );
        assert_eq!(
            app.events().len(),
            2,
            "the saving placement replaces the draft"
        );
        (
            app.writes[0].operation_id.clone(),
            app.writes[0].event_key.clone(),
        )
    });

    send(
        &feed,
        context,
        result(&first_operation, Err(WriteError::NotApplied)),
    );
    app.read_with(context, |app, _| {
        let editor = app.editor.as_ref().expect("the draft stays open");
        assert!(editor.in_flight.is_none());
        assert!(editor.status.as_deref().unwrap().contains("did not apply"));
        assert!(app.writes.is_empty());
        assert_eq!(app.events().len(), 2, "the draft is visible again");
    });

    app.update(context, |app, context| app.save_editor(context));
    app.read_with(context, |app, _| {
        assert_ne!(app.writes[0].operation_id, first_operation);
        assert_ne!(app.writes[0].event_key, first_key);
        assert_eq!(app.writes[0].placement.as_ref().unwrap().title, "Lunch");
    });
}

#[gpui::test]
fn stale_write_shows_the_newer_version(context: &mut TestAppContext) {
    let (feed, app, event, context) = world(context, true);
    let operation_id = submit(&app, context, Action::Update(moved(&event))).unwrap();
    let newer = CalendarEvent {
        title: "Renamed elsewhere".into(),
        revision: 1,
        ..event
    };
    send(
        &feed,
        context,
        Update::Snapshot(snapshot(std::slice::from_ref(&newer), true, Vec::new())),
    );
    send(
        &feed,
        context,
        result(&operation_id, Err(WriteError::Stale)),
    );
    app.read_with(context, |app, _| {
        assert_eq!(&*app.events(), std::slice::from_ref(&newer));
        assert_eq!(app.selected.as_ref(), Some(&newer));
        assert!(app.notices()[0].contains("Renamed elsewhere"));
    });
}
