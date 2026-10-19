# Contributing

Keep identifiers explicit: `event_id`, `calendar_id`, `operation_id`. Use Rust formatting and the checks below. Keep changes local unless publishing is explicitly requested.

After the initial repository bootstrap, make every change on a feature branch and open a pull request into `main`. Let CI finish and resolve failures before merging. Do not push changes directly to `main`. Creating a PR does not authorize merging it.

## Where changes belong

- `crates/calendar-core`: domain types, validation and time rules. No GUI, HTTP or database dependency.
- `crates/calendar-layout`: pure event placement and overlapping-column calculation.
- `apps/desktop/src/app.rs`: state and lifecycle; `apps/desktop/src/app/`: views, editor, gestures and controls.
- `apps/desktop/src/platform.rs`: native backdrop/tray placement. Keep Windows imports behind target gates.
- `apps/desktop/src/service.rs`: background GraphQL calls, cache and operation journal. See the module map in `docs/architecture.md` for web/mobile controllers and Google provider modules.
- `apps/backend/src/repository.rs`: storage transactions and conflict/idempotency rules.
- `crates/telemetry`: local diagnostics, PostHog events/errors and OpenTelemetry log/trace export.

## Testing

Use ordinary Rust tests for calendar mathematics, `#[tokio::test]` plus Tower `ServiceExt::oneshot` for real Axum route behavior, and `#[gpui::test]` with `TestAppContext` for GUI state transitions. Tests must use isolated memory databases or test-owned files, never the running preview database.

Add cases for behavior that can lose or misplace time: overlap clusters, midnight, leap dates, DST gaps/repeated hours, stale writes and repeated operation IDs. Test requested behavior rather than implementation details.

Before declaring native UI work verified, check opaque header/sidebar colors and the Activity panel, minimize/maximize/restore, resized layouts, drag/resize cancellation, offline event viewing, and tray activation/quit. Compilation and headless tests do not prove native appearance.

macOS builds need a Mac with the Xcode tools. Use normal `cargo` commands there; do not introduce Windows dependencies into shared crates. Keep traffic-light space, menu-bar template icons and native window lifecycle in mind. No macOS support claim until it has been built and exercised there.

## Logs and privacy

Log meaningful state changes with `action`, object/operation IDs and outcome. Never log OAuth credentials, session tokens, full API requests, or event content. Use `RUST_LOG` for filtering and `CALENDAR_LOG_DIR` to select a local directory. Cache/journal files contain event content; logs are diagnostics, not an audit backup.

References: [Axum's testing example](https://github.com/tokio-rs/axum/blob/main/examples/testing/src/main.rs), [GPUI test context](https://docs.rs/gpui/latest/gpui/attr.test.html).

For telemetry, use static screen/control/error categories. Instrument async operations with `#[tracing::instrument(skip_all)]` so event bodies and tokens never become span attributes. Remote logs use an allowlist; do not expand it to free-form user data. Keep transport logs out of remote export to prevent feedback loops. See [telemetry](docs/telemetry.md) for configuration and verification.

Document GraphQL fields and arguments alongside their Rust definitions. Keep examples in `docs/graphql.md` aligned with the exported SDL. Tests should cover authenticated introspection, query/mutation behavior and error extension codes.

## Readability and formatting

Use one statement per line. Separate imports from declarations, and separate functions, hooks, validation, side effects, and returns with a blank line. Keep related declarations together. Use explicit identifiers and extract coherent responsibilities rather than introducing one-line wrapper modules.

Run `bun run format` (Oxfmt) for maintained TypeScript, JavaScript, CSS, HTML, JSON, YAML, TOML, GraphQL and Markdown; use `bun run format:check` in CI. Run `bun run lint` (Oxlint) for the TypeScript/JavaScript workspace and `bun run typecheck` separately. Mobile uses the same root lint configuration. Run `./scripts/cargo.ps1 fmt --all` for Rust. Keep PowerShell statements in readable logical groups and validate scripts with the PowerShell parser. Generated operation types, route trees, GraphQL SDL, native projects, assets and build outputs are excluded from manual formatting. Oxfmt preserves logical blank lines but does not decide where they belong; review those boundaries explicitly.

Oxlint uses correctness rules with TypeScript and React support, including hook rules and dependency checks. It replaces the Expo ESLint preset rather than reproducing every Expo-specific rule. Rust source remains covered by rustfmt and Clippy. Both Oxc tools use small root JSON configs; keep generated and bundled files excluded.

Keep formatting commits separate from behavior-preserving module extractions. Refactoring must preserve OAuth scopes, storage keys, event IDs, optimistic updates, notification scheduling and platform boundaries. Run the affected browser/native checks after moving code.

## Stack and project structure

The desktop uses Rust/GPUI, the backend uses Axum/GraphQL, and the local preview uses SQLite. Web uses TanStack Start; mobile uses Expo/React Native. Bun workspaces and Turborepo coordinate TypeScript packages alongside the Cargo workspace.

```text
calendar/
|-- apps/
|   |-- backend/          Axum API, Google integration and storage
|   |-- desktop/          GPUI calendar and tray
|   |-- mobile/           Expo Android/iOS client
|   `-- web/              TanStack Start calendar and landing page
|-- crates/
|   |-- calendar-core/    Rust event and time rules
|   |-- calendar-layout/  Event placement and overlap calculations
|   `-- telemetry/        Diagnostics and optional remote telemetry
|-- packages/
|   |-- api/              Typed GraphQL client and shared query state
|   |-- calendar/         TypeScript calendar helpers
|   `-- design-tokens/    Shared design values
|-- contracts/graphql/   Schema and client operations
|-- assets/              Bundled fonts, icons and GraphiQL assets
|-- scripts/             Local development and packaging helpers
|-- tests/               Web and mobile browser tests
|-- docs/                Architecture and integration guides
|-- .github/workflows/   Checks and application builds
|-- .claude/             Claude rules and review agents
|-- .codex/              Codex rules and review agents
`-- AGENTS.md            Repository instruction entry point
```

See [architecture](docs/architecture.md) for ownership boundaries, [web development](docs/web.md) for the local transport, and [mobile development](docs/mobile.md) for device setup. The current backend is local; public multi-user hosting is not implemented.

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

## Checks

```powershell
./scripts/cargo.ps1 fmt --all '--' --check
./scripts/cargo.ps1 test --workspace --features calendar-desktop/ui-tests
./scripts/cargo.ps1 clippy --workspace --all-targets --features calendar-desktop/ui-tests '--' '-D' warnings
```

GPUI tests run against a test window, without operating your desktop. Native appearance, tray placement and pointer hit targets still need runtime inspection. macOS boundaries exist, but no macOS build has been verified here.

## Downloads and CI

GitHub Actions runs regular checks on every pull request, pushes to `main`, and manual workflow runs. Release/preview builds run only on pull requests targeting `main`, after the required checks pass. The Windows release build is uploaded as the `calendar-windows-x64-<commit>` artifact for 14 days. Download it from the workflow run and extract both ZIP layers if GitHub wraps the portable ZIP.

The portable package contains both executables. Copy `.env.example` to `.env`, configure your own Desktop OAuth credentials as described in [Google setup](docs/google.md), and run `Run-Calendar.ps1` with PowerShell. Keep the package in a writable folder. It requires neither Rust nor Visual Studio; the local backend remains on port 47821. These are unsigned preview builds, not signed installers. No account credentials, telemetry token or local calendar cache is packaged.

After the bootstrap commit, changes go through feature branches and PRs with passing CI. Regular checks cover formatting, Rust/GPUI tests, Clippy, generated GraphQL contracts, TypeScript, linting, shared date tests, and web/mobile browser interactions. PRs into feature branches skip application packaging. Only PRs into `main` also build the web production output, export mobile bundles, and package Windows executables, Android preview APKs, and unsigned iOS simulator archives. Test-related compilation still runs wherever the tests require it.

## GraphQL documentation

Open [GraphiQL](http://127.0.0.1:47821/docs), paste `.local/session-token` into the local session token field, then click Connect. Open the Docs panel to browse queries, mutations, types, field descriptions and arguments. The bundled UI runs locally and keeps credentials only in memory. The PostHog project token is unrelated to API authentication.

- `POST /graphql`: authenticated queries, mutations and introspection.
- `GET /schema.graphql`: public schema SDL (types and documentation only, no calendar data).
- `GET /health`: local backend health.
- `GET /diagnostics/logs`: authenticated recent backend diagnostics.

The backend exports the documented schema to `contracts/graphql/schema.graphql` at startup. See [GraphQL examples](docs/graphql.md) for queries, mutations and error handling. The temporary REST calendar endpoints and Scalar integration have been removed. Existing events, cache and mutation journal are preserved.

## Preview implementation notes

- Display time defaults to Asia/Jakarta. Dragging snaps to 15 minutes; all-day end dates are exclusive.
- Offline viewing uses account-specific SQLite caches. New offline edits are rejected; mutation IDs and a durable operation journal prepare for later reconciliation and queued edits.
- The local session token is a development guard, not multi-user authentication. Existing Google accounts may need to approve editing/profile permissions in Settings. Recurring-series edits and public onboarding remain future work.
- Activity & diagnostics shows desktop/backend entries with a warning/error filter and copy action. Daily JSON logs live in `.local/logs/`, retaining seven files per application. Logs contain operation/event IDs and outcomes, not event titles, descriptions or tokens.
- Windows previews are unsigned. macOS boundaries and iOS simulator builds do not establish physical-device verification.

## Agent instructions

`AGENTS.md` routes Codex to the relevant `.codex/rules/` documents. `.codex/agents/` contains TOML definitions for the two review roles, following the [official custom agent format](https://developers.openai.com/codex/subagents). Rules are read through explicit references; a `.codex/rules/` directory alone is not an automatic instruction loader. The reviewers inherit the session model and permissions and are instructed to inspect and report only. Their instructions are not a security sandbox.

Claude rule and agent equivalents remain in `.claude/`. Keep matching guidance aligned. The former root `CLAUDE.md` is now `AGENTS.md`; Claude users should load `AGENTS.md` explicitly when their client does not discover it.
