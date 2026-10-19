use super::*;

impl CalendarApp {
    pub(super) fn begin_gesture(
        &mut self,
        event: CalendarEvent,
        kind: GestureKind,
        position: Point<Pixels>,
        context: &mut Context<Self>,
    ) {
        if self.pending {
            return;
        }
        if (!self.online || event.read_only) && !matches!(kind, GestureKind::Move) {
            return;
        }
        self.gesture = Some(Gesture {
            kind,
            event,
            origin: position,
            changed: false,
        });
        context.notify();
    }

    pub(super) fn move_gesture(
        &mut self,
        position: Point<Pixels>,
        window: &Window,
        context: &mut Context<Self>,
    ) {
        if !self.online || self.pending {
            return;
        }
        let sidebar_width = self.sidebar_width();
        let Some(gesture) = &mut self.gesture else {
            return;
        };
        if gesture.event.read_only {
            return;
        }
        let day_width = (f32::from(window.viewport_size().width) - sidebar_width - GUTTER) / 7.0;
        let horizontal = f32::from(position.x - gesture.origin.x);
        let vertical = f32::from(position.y - gesture.origin.y);
        let minute_delta = snap_minutes(vertical * 60.0 / HOUR);
        let day_delta = (horizontal / day_width).round() as i64;
        if horizontal.abs() + vertical.abs() > 4.0 {
            gesture.changed = true;
        }
        let mut preview = gesture.event.clone();
        match gesture.kind {
            GestureKind::Move => {
                let local = preview.start.with_timezone(&self.timezone).naive_local()
                    + Duration::days(day_delta)
                    + Duration::minutes(minute_delta);
                match resolve_local(
                    local.date(),
                    local.hour() * 60 + local.minute(),
                    self.timezone,
                ) {
                    Ok(start) => {
                        let duration = preview.end - preview.start;
                        preview.start = start;
                        preview.end = start + duration;
                    }
                    Err(error) => {
                        self.message = error;
                        context.notify();
                        return;
                    }
                }
            }
            GestureKind::Start => {
                preview.start = (preview.start + Duration::minutes(minute_delta))
                    .min(preview.end - Duration::minutes(15));
            }
            GestureKind::End | GestureKind::Create => {
                preview.end = (preview.end + Duration::minutes(minute_delta))
                    .max(preview.start + Duration::minutes(15));
            }
        }
        self.events
            .retain(|event| event.event_id != preview.event_id);
        self.events.push(preview);
        context.notify();
    }

    pub(super) fn finish_gesture(&mut self, window: &mut Window, context: &mut Context<Self>) {
        let Some(gesture) = self.gesture.take() else {
            return;
        };
        let preview = self
            .events
            .iter()
            .find(|event| event.event_id == gesture.event.event_id)
            .cloned()
            .unwrap_or_else(|| gesture.event.clone());
        self.events
            .retain(|event| event.event_id != gesture.event.event_id);
        if gesture.event.revision >= 0 {
            self.events.push(gesture.event.clone());
        }
        if matches!(gesture.kind, GestureKind::Create) {
            self.edit(preview, window, context);
        } else if gesture.changed {
            calendar_telemetry::track(
                "event_gesture_completed",
                match gesture.kind {
                    GestureKind::Move => "move",
                    GestureKind::Start | GestureKind::End => "resize",
                    GestureKind::Create => "create",
                },
            );
            self.commit(preview, context);
        } else {
            self.edit(gesture.event, window, context);
        }
        context.notify();
    }
}
