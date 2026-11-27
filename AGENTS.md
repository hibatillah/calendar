# Calendar

Windows-only GPUI Google Calendar client that talks to Google directly; there is no backend, web or mobile app.

- Before changing code, read [CONTRIBUTING.md](CONTRIBUTING.md) for module ownership, naming, testing and logging conventions.
- For cross-layer changes, read [docs/architecture.md](docs/architecture.md). For Google integration, also read [docs/google.md](docs/google.md).
- Use [CONTRIBUTING.md](CONTRIBUTING.md) for current build/run commands. Executable workflows belong in `scripts/`.
- Always read `.claude/rules/project.md` and `.claude/rules/verification.md`. For calendar behavior, read `.claude/rules/calendar-behavior.md`; for desktop UI or assets, read `.claude/rules/desktop-ui.md`; for OAuth, sync or persistence, read `.claude/rules/google-and-data.md`. Treat rules as desired behavior, not proof that the implementation satisfies them.
- Keep implementation status in the existing docs. Avoid copying changing status or secret values into instructions.

## Subagents

**A change under an agent's paths MUST be dispatched to that agent through the Agent tool, not edited inline**, except a one-line fix or a typo. The main session plans, orders the areas, answers questions and owns commits. An agent without a subagent mechanism reads the matching file in `.claude/agents/` and follows it.

| Agent | Area |
| --- | --- |
| `sync-engineer` | `crates/calendar-google`, `crates/calendar-sync`, `crates/calendar-core`, `crates/desktop/src/service.rs` |
| `desktop-engineer` | `crates/desktop` except `service.rs`, `crates/calendar-layout`, `assets/`, `installer/`, `scripts/`, `.github/`, theme and `DESIGN.md` |
| `calendar-sync-reviewer` | read-only review of OAuth, multi-account isolation, sync, cache and write changes |
| `desktop-interaction-reviewer` | read-only review of GPUI interaction, header, sidebar, popover, settings and tray changes |

A task across areas goes in this order: `sync-engineer`, `desktop-engineer`, then the reviewer that matches. Give each agent a self-contained brief with the goal, the files to read and what to report. Agents that build run one at a time per source tree, each with its own `CARGO_TARGET_DIR`. Reviewers report; the main session applies fixes and runs checks. Each implementer ends its report with a Memory line; the owner measures memory on a Windows machine.

## Skills

The agents preload these skills by name. They are not installed yet; an agent whose skill is missing says so and continues with the docs.

| Skill | Source | Used by |
| --- | --- | --- |
| `rust-best-practices` | `apollographql/skills` | `sync-engineer`, `desktop-engineer` |
| `rust-patterns` | `affaan-m/ecc` | `sync-engineer`, `calendar-sync-reviewer` |
| `rust-testing` | `affaan-m/ecc` | `sync-engineer`, `desktop-engineer` |
| `gpui-kit` | `longbridge/gpui-kit` | `desktop-engineer`, `desktop-interaction-reviewer` |
