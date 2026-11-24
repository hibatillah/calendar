# Contributing

Keep identifiers explicit: `event_id`, `calendar_id`, `operation_id`. Use Rust formatting and the checks below. Keep changes local unless publishing is explicitly requested.

After the initial repository bootstrap, make every change on a feature branch and open a pull request into `main`. Let CI finish and resolve failures before merging. Do not push changes directly to `main`. Creating a PR does not authorize merging it.

Calendar is a Windows-only desktop app that talks to Google Calendar directly. There is no backend, web or mobile client. See [architecture](docs/architecture.md) for the target layout and the migration that is still in progress.

## Where changes belong

- `crates/calendar-core`: domain types, validation and time rules. No GUI, HTTP or database dependency.
- `crates/calendar-layout`: pure event placement and overlapping-column calculation.
- `crates/desktop/src/app.rs`: state and lifecycle; `crates/desktop/src/app/`: views, editor, gestures and controls.
- `crates/desktop/src/platform.rs`: tray and window placement. Keep Windows imports behind target gates.
- `crates/desktop/src/service.rs`: bridge between the UI and background work. Network and database work never runs on the UI thread.
- `crates/telemetry`: local diagnostics and optional PostHog/OpenTelemetry export.
- `crates/calendar-google` and `crates/calendar-sync` are planned; see the architecture document before adding Google or storage code elsewhere.

## Testing

Use ordinary Rust tests for calendar mathematics and `#[gpui::test]` with `TestAppContext` for GUI state transitions. Tests must use isolated memory databases or test-owned files, never the running app's database. Provider tests use fake HTTP endpoints, not Google.

Add cases for behavior that can lose or misplace time: overlap clusters, midnight, leap dates, DST gaps/repeated hours, stale writes and repeated operation IDs. Test requested behavior rather than implementation details.

Before declaring native UI work verified, check opaque header/sidebar colors and the Activity panel, minimize/maximize/restore, resized layouts, drag/resize cancellation, offline event viewing, and tray activation/quit. Compilation and headless tests do not prove native appearance.

## Logs and privacy

Log meaningful state changes with `action`, object/operation IDs and outcome. Never log OAuth credentials, tokens, full API requests, or event content. Use `RUST_LOG` for filtering and `CALENDAR_LOG_DIR` to select a local directory. Cache/journal files contain event content; logs are diagnostics, not an audit backup.

For telemetry, use static screen/control/error categories. Instrument async operations with `#[tracing::instrument(skip_all)]` so event bodies and tokens never become span attributes. Remote logs use an allowlist; do not expand it to free-form user data. See [telemetry](docs/telemetry.md).

## Readability and formatting

Use one statement per line. Separate imports from declarations, and separate functions, validation, side effects, and returns with a blank line. Use explicit identifiers and extract coherent responsibilities rather than introducing one-line wrapper modules. Run `bash scripts/cargo.sh fmt --all` for Rust. Validate shell scripts with `bash -n scripts/<name>.sh`.

Keep formatting commits separate from behavior-preserving module extractions. Refactoring must preserve OAuth scopes, storage keys, event IDs and optimistic updates.

## Stack and project structure

Rust and GPUI for the app, SQLite for the local cache, Cargo for builds. See [architecture](docs/architecture.md) for the full proposed layout.

```text
calendar/
|-- crates/
|   |-- desktop/          GPUI calendar and tray (the only binary)
|   |-- calendar-core/    Event and time rules
|   |-- calendar-layout/  Event placement and overlap calculations
|   `-- telemetry/        Diagnostics and optional remote telemetry
|-- assets/fonts/         Bundled Inter
|-- installer/windows/    WiX MSI authoring
|-- scripts/              Bash entry points (build, relaunch, watch, package)
|-- docs/                 Architecture, Google setup, telemetry
|-- .github/workflows/    Checks and Windows packaging
|-- .claude/, .codex/     Rules and review agents
`-- AGENTS.md             Repository instruction entry point
```

## Run locally

Use Bash entry points from Git Bash on Windows. Builds require Rust's MSVC toolchain, Visual Studio C++ Build Tools and the Windows SDK. All scripts are plain Bash; there are no PowerShell helpers. `scripts/cargo.sh` selects the `stable-x86_64-pc-windows-msvc` toolchain explicitly so Git's `link.exe` is never used; run Cargo through it. Process control and ZIP creation call `cmd.exe`/`powershell.exe` inline.

WSL runs Linux binaries, not the Windows desktop build. The shared crates (`calendar-core`, `calendar-layout`, `calendar-telemetry`) build and test on Linux; `calendar-desktop` needs Windows.

```bash
bash scripts/start.sh
```

For automatic GPUI rebuild/restart during development:

```bash
bash scripts/dev-desktop.sh
```

The watcher debounces changes to Rust sources, Cargo manifests and bundled assets, builds incrementally, keeps the last working app open when compilation fails, and restarts the app only when its binary changed. This is automatic rebuild/restart, not in-process hot reload: saved settings, accounts and caches survive, but unsaved forms and transient UI state reset. Run only one watcher per checkout. On Windows, Ctrl+C stops watching and leaves the app running; `--once` builds once.

Run commands from the repository root. State and logs go into ignored `.local/`.

## Checks

```bash
bash scripts/cargo.sh fmt --all '--' --check
bash scripts/cargo.sh test --workspace --features calendar-desktop/ui-tests
bash scripts/cargo.sh clippy --workspace --all-targets --features calendar-desktop/ui-tests '--' '-D' warnings
```

GPUI tests run against a test window, without operating your desktop. Native appearance, tray placement and pointer hit targets still need runtime inspection. Run `bash scripts/test-dev-desktop.sh` for watcher checks. Validate shell syntax with `bash -n scripts/<name>.sh`.

## Packaging and CI

GitHub Actions runs shell checks and Rust checks (format, tests including GPUI interactions, Clippy) on Windows for every pull request. PRs into `main` also build the release executable, a portable ZIP and an MSI, then install and uninstall the MSI on the disposable runner. The result is uploaded as `calendar-windows-x64-<commit>` for 14 days.

The MSI installs `calendar-desktop.exe` in Program Files, creates Start-menu and desktop shortcuts, and registers with Windows Settings → Apps. Configuration and caches live under `%LOCALAPPDATA%\Calendar`; edit the `.env` there for [Google setup](docs/google.md). Uninstall preserves user data and Credential Manager entries; disconnect accounts in Settings first to revoke access. The portable ZIP runs with `Run-Calendar.cmd`, which keeps data in `%LOCALAPPDATA%\Calendar` like the installed copy. The app itself still writes `.local/` relative to its working directory; resolving a data directory inside the app is part of the migration. Both are unsigned previews; no credentials, telemetry token or calendar cache is packaged.

MSI authoring is in `installer/windows/Calendar.wxs`. Install .NET SDK 8, then `dotnet tool install --global wix --version 6.0.2` and `wix extension add --global WixToolset.UI.wixext/6.0.2`. After building the executable, run `bash scripts/package-windows.sh` and `bash scripts/package-msi.sh --version 1.10.1`. Use increasing three-part MSI versions for upgrades; CI derives its version from UTC year/month/minute-of-month. `scripts/test-msi.sh` installs and uninstalls only in disposable CI runners.

Every push to `main` runs `.github/workflows/release.yml`, which reuses CI at the merged commit, then publishes a draft GitHub Release (`preview-<run number>`) with the ZIP, MSI and `SHA256SUMS.txt` and marks it as a prerelease. Release notes use GitHub's generated PR notes and `.github/release.yml` categories.

## Agent instructions

`AGENTS.md` routes Codex to the relevant `.codex/rules/` documents; `.codex/agents/` holds the two review roles. Claude equivalents live in `.claude/`. Keep matching guidance aligned across both directories.
