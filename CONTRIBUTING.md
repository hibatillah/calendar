# Contributing

Keep identifiers explicit: `event_id`, `calendar_id`, `operation_id`. Use Rust formatting and the checks below. Keep changes local unless publishing is explicitly requested.

After the initial repository bootstrap, make every change on a feature branch and open a pull request into `main`. Let CI finish and resolve failures before merging. Do not push changes directly to `main`. Creating a PR does not authorize merging it.

Calendar is a Windows-only desktop app that talks to Google Calendar directly. There is no backend, web or mobile client. See [architecture](docs/architecture.md) for the layout. Sync and event writes are tested against fakes only; live behavior with a real Google account is not yet verified.

## Where changes belong

- `crates/calendar-core`: domain types, validation and time rules. No GUI, HTTP or database dependency.
- `crates/calendar-layout`: pure event placement and overlapping-column calculation.
- `crates/desktop/src/app.rs`: state and lifecycle; `crates/desktop/src/app/`: views, editor, gestures, write overlay and controls.
- `crates/tray`: the always-running `calendar-tray.exe` (tray icon, menu, idle sync). No GPUI. It only starts the UI process and reads the cache.
- `crates/desktop/src/platform.rs`: window placement and focusing an existing window. Keep Windows imports behind target gates.
- `crates/desktop/src/service.rs` and `service/`: bridge between the UI and background work (commands, sync thread, sign-in and disconnect threads, snapshots). Network, database and credential-vault work never runs on the UI thread. Tests use an in-memory cache, a fake HTTP server and a test-owned Credential Manager service.
- `crates/calendar-sync/src/logging.rs`: local file logger shared by both processes. Nothing is sent anywhere except to Google.
- `crates/calendar-google`: Google OAuth (PKCE, loopback), token refresh, Calendar REST client and Google-to-core mapping. Blocking HTTP; no GUI, storage or async runtime. Tests use a local fake HTTP server.
- `crates/calendar-sync`: SQLite cache, sync tokens, write journal, sync engine, poll schedule and the Windows credential vault. Blocking; no GUI or async runtime. Tests use in-memory SQLite, a fake HTTP server and a test-owned Credential Manager service. All storage code lives here.

## Testing

Use ordinary Rust tests for calendar mathematics and `#[gpui::test]` with `TestAppContext` for GUI state transitions. Tests must use isolated memory databases or test-owned files, never the running app's database. Provider tests use fake HTTP endpoints, not Google.

Add cases for behavior that can lose or misplace time: overlap clusters, midnight, leap dates, DST gaps/repeated hours, stale writes and repeated operation IDs. Test requested behavior rather than implementation details.

Before declaring native UI work verified, check opaque header/sidebar colors, minimize/maximize/restore, resized layouts, drag/resize cancellation, offline event viewing, and tray activation/quit. Compilation and headless tests do not prove native appearance.

Event writes need these manual checks with a real account (none done yet): create by dragging an empty slot and with New; edit and delete from the popover; drag to move and resize (placement stays while saving, returns on failure, offline attempt shows the reason); a read-only calendar and a birthday refuse edits; one occurrence of a recurring event can be edited; popover and form scroll without moving the timeline and their buttons are clickable at 1050 x 680; date and time fields align with the grid; a write cut off by quitting shows as pending after restart.

## Logs and privacy

Log meaningful state changes with `action`, object/operation IDs and outcome. Never log OAuth credentials, tokens, full API requests, or event content. Logs go to `%LOCALAPPDATA%\Calendar\logs\calendar.log` (one file plus one rotated predecessor, about 1 MB each); set `RUST_LOG` to one level (`error`, `warn`, `info`, `debug`, `trace`; default `info`) and `CALENDAR_LOG_DIR` to select another local directory. Cache/journal files contain event content; logs are diagnostics, not an audit backup.

Nothing leaves the machine except calls to Google; there is no remote logging or analytics. Instrument async operations with `#[tracing::instrument(skip_all)]` so event bodies and tokens never become span attributes.

## Readability and formatting

Use one statement per line. Separate imports from declarations, and separate functions, validation, side effects, and returns with a blank line. Use explicit identifiers and extract coherent responsibilities rather than introducing one-line wrapper modules. Run `bash scripts/cargo.sh fmt --all` for Rust. Validate shell scripts with `bash -n scripts/<name>.sh`.

Keep formatting commits separate from behavior-preserving module extractions. Refactoring must preserve OAuth scopes, storage keys, event IDs and optimistic updates.

## Stack and project structure

Rust and GPUI for the app, SQLite for the local cache, Cargo for builds. See [architecture](docs/architecture.md) for the full layout.

```text
calendar/
|-- crates/
|   |-- desktop/          GPUI calendar UI (calendar-desktop.exe)
|   |-- tray/             tray icon, menu and idle sync (calendar-tray.exe)
|   |-- calendar-core/    Event and time rules
|   |-- calendar-google/  Google OAuth, Calendar REST client, mapping
|   |-- calendar-sync/    SQLite cache, sync engine and schedule, credential vault
|   `-- calendar-layout/  Event placement and overlap calculations
|-- assets/fonts/         Bundled Inter
|-- installer/windows/    WiX MSI authoring
|-- scripts/              Bash entry points (build, relaunch, watch, package)
|-- docs/                 Architecture, Google setup
|-- .github/workflows/    Checks and Windows packaging
|-- .claude/            Rules and agents
`-- AGENTS.md             Repository instruction entry point
```

## Run locally

Use Bash entry points from Git Bash on Windows. Builds require Rust's MSVC toolchain, Visual Studio C++ Build Tools and the Windows SDK. All scripts are plain Bash; there are no PowerShell helpers. `scripts/cargo.sh` selects the `stable-x86_64-pc-windows-msvc` toolchain explicitly so Git's `link.exe` is never used; run Cargo through it. Process control and ZIP creation call `cmd.exe`/`powershell.exe` inline.

WSL runs Linux binaries, not the Windows desktop build. The shared crates (`calendar-core`, `calendar-layout`) build and test on Linux; `calendar-desktop` needs Windows.

```bash
bash scripts/start.sh
```

For automatic GPUI rebuild/restart during development:

```bash
bash scripts/dev-desktop.sh
```

The watcher debounces changes to Rust sources, Cargo manifests and bundled assets, builds incrementally, keeps the last working app open when compilation fails, and restarts the app only when its binary changed. This is automatic rebuild/restart, not in-process hot reload: saved settings, accounts and caches survive, but unsaved forms and transient UI state reset. Run only one watcher per checkout. On Windows, Ctrl+C stops watching and leaves the app running; `--once` builds once.

Run commands from the repository root. State goes into ignored `.local/`; logs go to the directory above.

## Checks

```bash
bash scripts/cargo.sh fmt --all '--' --check
bash scripts/cargo.sh test --workspace --features calendar-desktop/ui-tests
bash scripts/cargo.sh clippy --workspace --all-targets --features calendar-desktop/ui-tests '--' '-D' warnings
```

GPUI tests run against a test window, without operating your desktop. Native appearance, tray placement and pointer hit targets still need runtime inspection. Run `bash scripts/test-dev-desktop.sh` for watcher checks. Validate shell syntax with `bash -n scripts/<name>.sh`.

## Packaging and CI

GitHub Actions runs shell checks and Rust checks (format, tests including GPUI interactions, Clippy) on Windows for every pull request. PRs into `main` also build the release executable, a portable ZIP and an MSI, then install and uninstall the MSI on the disposable runner. The result is uploaded as `calendar-windows-x64-<commit>` for 14 days.

The MSI installs `calendar-tray.exe` and `calendar-desktop.exe` side by side in Program Files (the tray starts the window from its own folder), creates Start-menu and desktop shortcuts that run `Run-Calendar.cmd` (starts the tray, then opens the window; a second copy of either exits or focuses the first), and registers with Windows Settings → Apps. Install, upgrade and uninstall close a running tray or window first (WiX `CloseApplication`). There is no start-with-Windows entry yet. Data lives under `%LOCALAPPDATA%\Calendar` (cache, `preferences.json`, logs); uninstall preserves it and the Credential Manager entries, so disconnect accounts in Settings first to revoke access. The portable ZIP holds the same two executables and runs with `Run-Calendar.cmd`. Both are unsigned previews and contain no calendar cache.

Google sign-in credentials are compiled in at build time from `GOOGLE_CLIENT_ID` and `GOOGLE_CLIENT_SECRET` ([setup](docs/google.md)). CI embeds the repository secrets of those names only for builds from `main` (the release); pull-request artifacts are built without them. A build without them still succeeds and reports that sign-in is not configured. The scripts never print the values; keep `set -x` and verbose Cargo output away from the release build.

MSI authoring is in `installer/windows/Calendar.wxs`. Install .NET SDK 8, then `dotnet tool install --global wix --version 6.0.2` and `wix extension add --global WixToolset.UI.wixext/6.0.2` and `wix extension add --global WixToolset.Util.wixext/6.0.2`. After building the executable, run `bash scripts/package-windows.sh` and `bash scripts/package-msi.sh --version 1.10.1`. Use increasing three-part MSI versions for upgrades; CI derives its version from UTC year/month/minute-of-month. `scripts/test-msi.sh` installs and uninstalls only in disposable CI runners.

Every push to `main` runs `.github/workflows/release.yml`, which reuses CI at the merged commit (inheriting the repository secrets), then publishes a draft GitHub Release (`preview-<run number>`) with the ZIP, MSI and `SHA256SUMS.txt` and marks it as a prerelease. Release notes use GitHub's generated PR notes and `.github/release.yml` categories.

## Agent instructions

`AGENTS.md` routes agents to the relevant `.claude/rules/` documents; `.claude/agents/` holds the implementer and review subagents.
