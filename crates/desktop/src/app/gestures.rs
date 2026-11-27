//! Drag to move or resize an event, and drag on an empty day to create one.
//!
//! A gesture only builds a preview; the write happens when the mouse is released.

use super::*;
use calendar_core::resolve_local;
use calendar_layout::snap_minutes;
use writes::Action;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum GestureKind {
    Move,
    Resize,
    Create,
}

pub(super) struct Gesture {
    kind: GestureKind,
    /// The event as it was (the draft for a create).
    event: CalendarEvent,
    origin: Point<Pixels>,
    pub preview: Option<CalendarEvent>,
}

const DRAG_THRESHOLD: f32 = 4.0;
const MINIMUM_MINUTES: i64 = 15;

impl CalendarApp {
    pub(super) fn day_width(&self, window: &Window) -> f32 {
        (f32::from(window.viewport_size().width) - self.sidebar_width() - GUTTER).max(560.0) / 7.0
    }

    pub(super) fn begin_gesture(
        &mut self,
        kind: GestureKind,
        event: CalendarEvent,
        position: Point<Pixels>,
        context: &mut Context<Self>,
    ) {
        self.gesture = Some(Gesture {
            kind,
            event,
            origin: position,
            preview: None,
        });
        context.notify();
    }

    /// Mouse down on an empty part of the day grid: start a draft at that time.
    pub(super) fn begin_create(
        &mut self,
        position: Point<Pixels>,
        window: &Window,
        context: &mut Context<Self>,
    ) {
        let bounds = self.scroll.bounds();
        let day_width = self.day_width(window);
        let x =
            f32::from(position.x - bounds.origin.x) - GUTTER + self.horizontal_fraction * day_width;
        let y = f32::from(position.y - bounds.origin.y - self.scroll.offset().y);
        let day = ((x / day_width).floor() as i64).clamp(0, 7);
        let minute = snap_minutes(y * 60.0 / HOUR).clamp(0, 1440 - MINIMUM_MINUTES) as u32;
        let date = self.date + Duration::days(day);
        match self.draft(date, minute, 60) {
            Ok(draft) => self.begin_gesture(GestureKind::Create, draft, position, context),
            Err(reason) => {
                self.write_notice = Some(reason);
                context.notify();
            }
        }
    }

    pub(super) fn move_gesture(
        &mut self,
        position: Point<Pixels>,
        window: &Window,
        context: &mut Context<Self>,
    ) {
        let day_width = self.day_width(window);
        let timezone = self.timezone;
        let Some(gesture) = &self.gesture else {
            return;
        };
        let horizontal = f32::from(position.x - gesture.origin.x);
        let vertical = f32::from(position.y - gesture.origin.y);
        if horizontal.abs() + vertical.abs() <= DRAG_THRESHOLD {
            return;
        }
        let kind = gesture.kind;
        let original = gesture.event.clone();
        if kind != GestureKind::Create && !self.can_write(&original) {
            self.write_notice = Some(calendar_sync::WriteError::ReadOnly.to_string());
            self.gesture = None;
            context.notify();
            return;
        }

        let minutes = snap_minutes(vertical * 60.0 / HOUR);
        let days = (horizontal / day_width).round() as i64;
        let mut preview = original.clone();
        match kind {
            GestureKind::Move => {
                let local = original.start.with_timezone(&timezone).naive_local()
                    + Duration::days(days)
                    + Duration::minutes(minutes);
                match resolve_local(local.date(), local.hour() * 60 + local.minute(), timezone) {
                    Ok(start) => {
                        preview.start = start;
                        preview.end = start + (original.end - original.start);
                    }
                    Err(reason) => {
                        self.write_notice = Some(reason);
                        context.notify();
                        return;
                    }
                }
            }
            GestureKind::Resize | GestureKind::Create => {
                preview.end = (original.end + Duration::minutes(minutes))
                    .max(original.start + Duration::minutes(MINIMUM_MINUTES));
            }
        }
        if let Some(gesture) = &mut self.gesture {
            gesture.preview = Some(preview);
        }
        context.notify();
    }

    pub(super) fn finish_gesture(&mut self, window: &mut Window, context: &mut Context<Self>) {
        let Some(gesture) = self.gesture.take() else {
            return;
        };
        match (gesture.kind, gesture.preview) {
            (GestureKind::Create, preview) => {
                self.open_editor(preview.unwrap_or(gesture.event), true, window, context);
            }
            (_, Some(preview)) => {
                if let Err(reason) = self.submit(Action::Update(preview), context) {
                    self.write_notice = Some(reason);
                }
            }
            (_, None) => self.show_details(gesture.event, context),
        }
        context.notify();
    }
}
