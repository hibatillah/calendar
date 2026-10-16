# Calendar

A native calendar, starting with Windows and designed to keep platform code separate for macOS. Rust/GPUI desktop, Axum/GraphQL backend, and SQLite for this entirely local preview.

The TanStack Start web app uses Bun workspaces and Turborepo alongside Cargo. See [web development](docs/web.md) for setup and the current local-backend boundary.

## Run locally

Prerequisites: Rust's MSVC toolchain, Visual Studio C++ Build Tools, Windows SDK, and PowerShell 7. The helper chooses MSVC without changing your default Rust installation.

```powershell
./scripts/start.ps1
```

The script builds both applications, starts the backend at `127.0.0.1:47821`, and opens the desktop. For separate terminals:

```powershell
./scripts/cargo.ps1 run -p calendar-backend
./scripts/cargo.ps1 run -p calendar-desktop
```

For automatic GPUI rebuild/restart during development:

```powershell
./scripts/dev-desktop.ps1
```

The watcher debounces changes to Rust sources, Cargo manifests and bundled assets. It builds incrementally, keeps the last working app open when compilation fails, and restarts only binaries whose contents changed. Backend-only changes do not restart the desktop. Changes saved during compilation trigger another build. A one-time handoff moves any existing debug processes to copies in `.local/dev-desktop/`, releasing Windows executable locks while preserving Cargo's warm cache. Run only one watcher per checkout; avoid `start.ps1` or concurrent Cargo builds while it is active.

This is automatic rebuild/restart, not in-process hot reload: saved settings, accounts and caches survive, but unsaved forms, scroll position and transient UI state reset. Avoid saving source while an event write is pending. Ctrl+C stops watching and leaves the apps running. `-Once` builds/restarts once for validation. PowerShell 7 is also usable on macOS, but this watcher has only been exercised on Windows.

Run commands from the repository root. State and logs go into ignored `.local/`. Protected API endpoints requires the local token in `.local/session-token`; it is not intended to be exposed to the network. There is no hosted backend. Optional PostHog uploads are disabled until explicitly configured; see [telemetry](docs/telemetry.md).

## Current preview

- Full-height opaque gray sidebar with bundled Inter, compact black/gray window chrome, month navigator and seven-day timeline.
- Week/month selector with **W** and **M** shortcuts (disabled in the event editor). Month cells show events; click an empty date to create one or an event to edit it.
- Collapsible sidebar; the reopen icon moves to the left edge of the main header. Simultaneous events slightly overlap within their day.
- Event changes render immediately while saving, and roll back only on failure.
- Google events in week/month/tray views, calendar colors and visibility toggles, recurring instances, all-day and multi-day display. Offline viewing uses an account-specific cache.
- Event editor with dates, times, notes, location and calendar selection.
- System tray agenda and Open/Quit menu. Closing the calendar hides it when the tray is available.
- SQLite cache for offline viewing. New edits are rejected offline; mutation IDs and a durable operation journal prepare for later queued edits.
- Live **Activity & diagnostics** panel in the sidebar: desktop/backend entries, warning/error filter and copy diagnostics.
- Daily JSON logs in `.local/logs/`, retaining seven files per application. Logs record operation/event IDs and outcomes, not event titles, descriptions, or access tokens.

**Google calendars support event creation and editing, including optimistic drag/resize.** Multiple accounts, discovery and bounded-range sync are available; see [Google setup](docs/google.md). Existing accounts must approve editing/profile permissions in the profile menu → Settings → Google accounts. Sample data is removed on startup. Recurring-series editing and public onboarding remain future work. The local session token is a development guard, not a multi-user authentication system.

Scroll sideways with a trackpad, horizontal wheel, or **Shift + mouse wheel** to move freely across dates. Headers and events stay aligned, and the sidebar highlights the seven dates centered in the viewport, excluding clipped edge slivers. Click a sidebar date to start the range there; Today resets to the current week using the configured week start. Regular vertical scrolling moves through the hours.

## Downloads and CI

GitHub Actions runs formatting, Rust/GPUI tests and Clippy on pull requests and pushes to `main`. A successful check job enables a Windows release build, uploaded as the `calendar-windows-x64-<commit>` artifact for 14 days. Download it from the workflow run and extract both ZIP layers if GitHub wraps the portable ZIP.

The portable package contains both executables. Copy `.env.example` to `.env`, configure your own Desktop OAuth credentials as described in [Google setup](docs/google.md), and run `Run-Calendar.ps1` with PowerShell. Keep the package in a writable folder. It requires neither Rust nor Visual Studio; the local backend remains on port 47821. These are unsigned preview builds, not signed installers. No account credentials, telemetry token or local calendar cache is packaged.

After the bootstrap commit, changes go through feature branches and PRs with passing CI. The web workflow checks generated GraphQL contracts, TypeScript, shared date tests, browser interactions and the production build.

## License

Original project code is under the [MIT license](LICENSE). Bundled Inter and GraphiQL/React assets retain their included third-party licenses.

Display time zone currently defaults to Asia/Jakarta. Dragging snaps to 15 minutes; Escape cancels a gesture or closes the editor. All-day end dates are exclusive. The month sidebar always includes every date, rather than assuming every month has 30 days.

## Checks

```powershell
./scripts/cargo.ps1 fmt --all '--' --check
./scripts/cargo.ps1 test --workspace --features calendar-desktop/ui-tests
./scripts/cargo.ps1 clippy --workspace --all-targets --features calendar-desktop/ui-tests '--' '-D' warnings
```

GPUI tests run against a test window, without operating your desktop. Native appearance, tray placement and pointer hit targets still need runtime inspection. macOS boundaries exist, but no macOS build has been verified here.

See [CONTRIBUTING.md](CONTRIBUTING.md) for the workflow and [architecture](docs/architecture.md) for ownership and sync design.




## GraphQL documentation

Open [GraphiQL](http://127.0.0.1:47821/docs), paste `.local/session-token` into the local session token field, then click Connect. Open the Docs panel to browse queries, mutations, types, field descriptions and arguments. The bundled UI runs locally and keeps credentials only in memory. The PostHog project token is unrelated to API authentication.

- `POST /graphql`: authenticated queries, mutations and introspection.
- `GET /schema.graphql`: public schema SDL (types and documentation only, no calendar data).
- `GET /health`: local backend health.
- `GET /diagnostics/logs`: authenticated recent backend diagnostics.

The backend exports the documented schema to `contracts/graphql/schema.graphql` at startup. See [GraphQL examples](docs/graphql.md) for queries, mutations and error handling. The temporary REST calendar endpoints and Scalar integration have been removed. Existing events, cache and mutation journal are preserved.
