# Architecture

Calendar is a Windows desktop application made of two processes, a small tray process and a GPUI UI process, talking to the Google Calendar REST API directly. There is no backend service, no web client and no mobile client.

## Status

The app reads and writes Google Calendar through `calendar-google` (blocking `ureq`) and `calendar-sync` (SQLite cache, no async runtime). **Implemented, not verified against a real Google account**: everything is tested against fake HTTP only. Local reminders and start with Windows are not built.

What the desktop does today: `service.rs` starts one sync thread (the `calendar-sync` scheduler, always in `WindowOpen` mode because the process exists only while its window is open) and publishes cached events to the UI before the first network call, then after every sync. Settings connect an account (browser consent on a short-lived thread, five-minute timeout, cancel, duplicate account refreshes the stored credential) and disconnect it (credential, cached rows, then a best-effort revoke). The views write through `Handle::write` (create, edit, move, resize, delete; the write path is in `calendar-sync`, tested against fakes only). Edits are overlaid on the snapshot at render time (`app/writes.rs`), so a background snapshot never overwrites a pending edit and a failure just drops the overlay. Avatars are initials.

The cache covers about now-30 days to now+180 days. When the displayed week or month reaches outside that range the view says so ("Only X to Y is downloaded. An empty day outside it is unknown, not free."); it never shows an unsynced date as an empty, healthy one. Refresh failures keep the saved events and name the account and the reason; a build without OAuth credentials says "Sign-in is not configured in this build."

`calendar-sync` layout: `store.rs` (rusqlite cache: accounts, calendars with `sync_token` and covered window, events with ETag, operation journal in `store/journal.rs`), `sync.rs` (`Syncer`: calendar list, bounded full sync, incremental sync, 410 resync) and `sync/write.rs` (event create, update, delete: journal, `If-Match`, settling unknown outcomes; see [Google](google.md#writes-calendar-syncsyncwrite)), `scheduler.rs` (`run`: poll interval and trigger rules on one thread), `vault.rs` (Windows only: Credential Manager). It uses `rusqlite` 0.43 with only the `bundled` feature (no system SQLite is available to link on Windows; about 1.1 MB added to the release binary, measured on a probe executable with the repository's release profile). The cache window and `syncToken` design is in [Google](google.md#cache-window-and-sync-tokens-calendar-sync).

`calendar-google` layout: `lib.rs` (config, errors, blocking client), `sign_in.rs` (PKCE, loopback listener, browser launch), `auth.rs` (token exchange, refresh, revoke, permissions), `api.rs` (calendar list, `syncToken` event reads, conditional writes), `mapping.rs` (Google ↔ `CalendarEvent`), `tests/` (fake HTTP server and flows). It uses `ureq` 3 (`rustls`, `json`; blocking, no async runtime), so the sync engine runs on one plain background thread; no async runtime is a direct dependency (`tokio` is in the lockfile only through GPUI).

## Target runtime shape

```text
UI process: GPUI window ◄── snapshots (async channel → Model entity) ──┐   tray process: tray icon + menu
     │ commands (Handle)                                    │        │ starts the UI with an argument
     ▼                                                      │        ▼
 service.rs (Handle + sync thread) ─────────────────────────┴──► sync engine (one plain background thread, blocking HTTP)
                                      │            │
                          calendar-google      store (SQLite cache, sync tokens, journal)
                          REST + OAuth         Credential Manager (refresh tokens)
```

- OAuth 2.0 authorization-code flow with PKCE and a loopback redirect on `127.0.0.1`. Refresh tokens live in Windows Credential Manager, access tokens in memory only.
- Reads use `events.list` with `syncToken` after the first full sync. A `410 Gone` clears that calendar's token and triggers a full resync. Writes use stable event IDs, operation ids and ETags (`If-Match`); they run on the sync thread (`Handle::write` queues a request and wakes the scheduler with `Message::Run`), the result arrives as `Update::Write`, and unknown outcomes appear in `Snapshot::pending` until the next sync settles them.
- `events.watch` is not used: it needs a public HTTPS endpoint. Sync is polled.
- The UI never performs network or database I/O. It shows cached data first and re-renders when a sync result arrives.

### Sync triggers

| State                         | Behavior                                              |
| ----------------------------- | ----------------------------------------------------- |
| Window open                   | Poll every 30–60 s                                    |
| Window closed, tray process only | Tray polls every 5 min; skipped while the UI process runs |
| Window opened, window focused, tray "Sync now" | One immediate sync                          |
| Sync finished a few seconds ago | Skip the trigger                                    |
| Sync already running          | Join it; never start a second one                     |
| Event write queued            | Runs on the same thread at once, ahead of the next poll |

Intervals are initial values to tune against measured request volume and battery impact.

### Two processes

`calendar-tray.exe` is always running: tray icon, menu, one worker thread (the scheduler in `TrayIdle` mode, one wake per 5 minutes) and no GPUI, no async runtime and no timers. It starts `calendar-desktop.exe` next to itself on demand: `--agenda` (left click), no argument (Open Calendar), `--settings`. The UI process owns syncing while it runs and exits when its last window closes, which releases all GPUI memory; GPUI does not hide windows on Windows, so close-to-tray was not an option. Each process holds a named mutex (`calendar-sync::instance`): a second UI process only focuses the first one's window, and before every poll the tray checks the UI's mutex and skips the sync (it still re-reads the cache for the "Next" item and tooltip). Cross-process safety is SQLite WAL with short transactions; there is no other IPC. A UI started by hand without the tray works the same; nothing then shows a tray icon until `calendar-tray.exe` runs. Memory numbers are under [Measured memory](#measured-memory).

## Repository layout

```text
calendar/
├── Cargo.toml                  workspace: members = ["crates/*"], release profile
├── crates/
│   ├── tray/                   calendar-tray.exe: tray icon, menu, idle sync (no GPUI)
│   ├── desktop/                calendar-desktop.exe: the GPUI UI
│   │   └── src/
│   │       ├── main.rs         startup, single instance, launch argument
│   │       ├── app.rs, app/    GPUI state, views, gestures, editor, write overlay
│   │       ├── platform.rs     window placement, focus an existing window, Windows-only APIs
│   │       └── service.rs      channel bridge between UI and sync engine
│   ├── calendar-core/          domain types and time rules (no GPUI/HTTP/storage)
│   ├── calendar-layout/        overlap and column layout (pure)
│   ├── calendar-google/        Google REST client, OAuth/PKCE, token refresh,
│   │                           Google ↔ core mapping. No GPUI, no storage.
│   ├── calendar-sync/          SQLite cache, sync tokens, operation journal,
│   │                           credential vault, sync engine and scheduler
├── assets/fonts/               bundled Inter + licence
├── installer/windows/          WiX MSI authoring + Run-Calendar.cmd launcher
├── scripts/                    plain Bash only (cargo, start, dev-desktop, package-*, test-*)
├── docs/                       architecture, google setup
├── .github/workflows/          ci.yml, release.yml
├── .claude/                    rules and agents
└── AGENTS.md
```

`calendar-google` and `calendar-sync` are separate from `crates/desktop` so they test with a fake HTTP server and in-memory SQLite, and so the domain and layout crates stay free of GPUI, HTTP and storage.

## Measured memory

Release build, 60 s after start, median of 3, no Google account, `scripts/measure-memory.sh`, 2026-11-27:

| State | Working set | Private |
| --- | --- | --- |
| Tray process idle | 11.1 MiB | 1.9 MiB |
| UI process, window open, idle | 55.3 MiB | 68.9 MiB |

Not measured yet: the UI after a sync with real data, and the tray after the UI window was closed.

## Remaining work

Verify connect, sync, writes and disconnect with a real Google account (checklists in [Google](google.md) and [Contributing](../CONTRIBUTING.md)), then local reminders.

## Diagnostics

Nothing leaves the machine except calls to Google; there is no remote logging or analytics. `crates/calendar-sync/src/logging.rs` (used by both processes) writes `tracing` events to a size-capped local file under `%LOCALAPPDATA%\Calendar\logs` (`CALENDAR_LOG_DIR` overrides it). Logs never contain tokens, consent URLs or event content.

## Time and identifiers

Calendar types use Chrono + chrono-tz. Named-zone conversion is explicit; nonexistent and ambiguous local times produce errors. Events store UTC instants and a named zone. All-day values use midnight boundaries in the display zone (the Windows system zone, recorded in the cache and shared by the window and the tray; see docs/google.md) with an all-day flag, preserving Google's exclusive end dates. Account-prefixed identifiers keep shared calendars from different accounts separate.

## UI module map

`app.rs` owns state and lifecycle; `app/shell.rs` composes the window. `app/navigation.rs`, `details.rs` (event popover), `editor.rs` (create and edit form), `gestures.rs` (drag to move, resize, create), `writes.rs` (pending placement and write outcomes), `notices.rs` (empty, stale and outside-the-cache messages), `timeline.rs`, `sidebar.rs`, `settings.rs`, `working_hours.rs` (the Working hours page, stored in `preferences.json`, and the off-hours shading behind the week grid; month view is unchanged), `accounts.rs` and `agenda.rs` own their interactions and rendering. `service.rs` (commands, sync and sign-in threads, `Model`) and `service/snapshot.rs` (cache to view types) sit below the views; the views copy the shared `Model` entity when it changes and send commands through `service::Handle`. Shared element builders live in `controls.rs`; headless interaction tests are in `app/tests.rs`, `app/write_tests.rs` and `app/working_hours_tests.rs`.
