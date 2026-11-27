//! When to sync. One plain thread blocks on a channel with a timeout equal to the time until the
//! next poll: no busy loop and no fixed short timer.
//!
//! The caller says which process it is ([`Mode`]) and sends [`Message::SyncNow`] for window open,
//! window focus and tray-menu open. A sync runs on the scheduler's own thread, so a second one can
//! never start while one is in flight; triggers that arrived meanwhile are answered by that
//! sync's result, because a trigger within `MIN_GAP` of a finished sync is skipped.

use std::{
    sync::mpsc::{Receiver, RecvTimeoutError},
    time::{Duration, Instant},
};

/// Poll interval while the window is open. Starting value, to tune against request volume.
const OPEN_INTERVAL: Duration = Duration::from_secs(60);
/// Poll interval while only the tray icon is running. Starting value, to tune.
const IDLE_INTERVAL: Duration = Duration::from_secs(300);
/// A trigger this soon after a finished sync is skipped.
const MIN_GAP: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    WindowOpen,
    TrayIdle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Message {
    /// Window opened, window focused or tray menu opened.
    SyncNow,
    /// Work is queued for the callback (an event write): run it now, even right after a sync.
    Run,
    Stop,
}

struct Schedule {
    mode: Mode,
    last_finished: Option<Instant>,
}

impl Schedule {
    /// Time until the next poll is due; zero when nothing has synced yet.
    fn wait(&self, now: Instant) -> Duration {
        let interval = match self.mode {
            Mode::WindowOpen => OPEN_INTERVAL,
            Mode::TrayIdle => IDLE_INTERVAL,
        };

        self.last_finished.map_or(Duration::ZERO, |finished| {
            (finished + interval).saturating_duration_since(now)
        })
    }

    /// False when a sync finished a few seconds ago, which also covers triggers that arrived while it ran.
    fn trigger_is_useful(&self, now: Instant) -> bool {
        self.last_finished
            .is_none_or(|finished| now.saturating_duration_since(finished) >= MIN_GAP)
    }
}

/// Syncs once at start, then on every poll interval and useful trigger, until `Stop` or until the
/// sender is dropped. Runs on the calling thread.
pub fn run(mode: Mode, messages: &Receiver<Message>, mut sync: impl FnMut()) {
    let mut schedule = Schedule {
        mode,
        last_finished: None,
    };
    let mut due = true;

    loop {
        if due {
            sync();
            schedule.last_finished = Some(Instant::now());
        }

        due = match messages.recv_timeout(schedule.wait(Instant::now())) {
            Ok(Message::SyncNow) => schedule.trigger_is_useful(Instant::now()),
            Ok(Message::Run) => true,
            Ok(Message::Stop) | Err(RecvTimeoutError::Disconnected) => return,
            Err(RecvTimeoutError::Timeout) => true,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, sync::mpsc};

    fn schedule(mode: Mode, finished: Option<Instant>) -> Schedule {
        Schedule {
            mode,
            last_finished: finished,
        }
    }

    #[test]
    fn polls_every_minute_with_the_window_open_and_every_five_in_the_tray() {
        let start = Instant::now();
        let seconds = Duration::from_secs;

        assert_eq!(schedule(Mode::WindowOpen, None).wait(start), Duration::ZERO);
        assert_eq!(
            schedule(Mode::WindowOpen, Some(start)).wait(start + seconds(10)),
            seconds(50)
        );
        assert_eq!(
            schedule(Mode::TrayIdle, Some(start)).wait(start + seconds(10)),
            seconds(290)
        );
        // A poll that is overdue is due at once.
        assert_eq!(
            schedule(Mode::WindowOpen, Some(start)).wait(start + seconds(200)),
            Duration::ZERO
        );
    }

    #[test]
    fn a_trigger_right_after_a_finished_sync_is_skipped() {
        let start = Instant::now();
        let after = |seconds| start + Duration::from_secs(seconds);
        let finished = schedule(Mode::WindowOpen, Some(start));

        assert!(!finished.trigger_is_useful(after(0)));
        assert!(!finished.trigger_is_useful(after(4)));
        assert!(finished.trigger_is_useful(after(5)));
        assert!(schedule(Mode::TrayIdle, None).trigger_is_useful(start));
    }

    #[test]
    fn triggers_queued_before_or_during_a_sync_join_it_instead_of_starting_another() {
        let (sender, messages) = mpsc::channel();
        sender.send(Message::SyncNow).unwrap();
        sender.send(Message::SyncNow).unwrap();
        let runs = Cell::new(0);

        run(Mode::WindowOpen, &messages, || {
            runs.set(runs.get() + 1);
            // Window focus and tray menu arrive while this sync is running.
            sender.send(Message::SyncNow).unwrap();
            sender.send(Message::Stop).unwrap();
        });

        assert_eq!(runs.get(), 1);
    }

    #[test]
    fn queued_work_runs_at_once_even_right_after_a_sync() {
        let (sender, messages) = mpsc::channel();
        sender.send(Message::Run).unwrap();
        sender.send(Message::Stop).unwrap();
        let runs = Cell::new(0);

        run(Mode::WindowOpen, &messages, || runs.set(runs.get() + 1));

        assert_eq!(runs.get(), 2, "the start-up sync, then the queued work");
    }

    #[test]
    fn stops_when_the_sender_is_dropped() {
        let (sender, messages) = mpsc::channel();
        drop(sender);
        let runs = Cell::new(0);

        run(Mode::TrayIdle, &messages, || runs.set(runs.get() + 1));

        assert_eq!(runs.get(), 1);
    }
}
