# Calendar

Windows-first GPUI calendar with an Axum/GraphQL backend. Preserve macOS boundaries. The TanStack Start client and shared TypeScript packages use Bun/Turborepo. Read `docs/web.md` before changing client transport or authentication, and `docs/mobile.md` for Expo/device builds.

- Before changing code, read [CONTRIBUTING.md](CONTRIBUTING.md) for module ownership, naming, testing and logging conventions.
- For cross-layer changes, read [docs/architecture.md](docs/architecture.md). For Google integration, also read [docs/google.md](docs/google.md).
- Use [CONTRIBUTING.md](CONTRIBUTING.md) for current build/run commands. Executable workflows belong in `scripts/`.
- Always read `.codex/rules/project.md` and `.codex/rules/verification.md`. For calendar behavior, read `.codex/rules/calendar-behavior.md`; for desktop UI or assets, read `.codex/rules/desktop-ui.md`; for OAuth, sync, persistence, GraphQL or telemetry, read `.codex/rules/google-and-data.md`. These are explicit document references, not automatically loaded Codex rule files. Treat rules as desired behavior, not proof that the implementation satisfies them.
- For desktop interaction reviews, use `desktop-interaction-reviewer`. For account, sync or persistence reviews, use `calendar-sync-reviewer`. Supply the changed paths and available test evidence. These agents inspect and report; the main agent owns fixes, test execution and runtime verification.
- Keep implementation status in the existing docs. Avoid copying changing status or secret values into instructions.
- Codex reviewer definitions live in `.codex/agents/*.toml`; Claude equivalents remain in `.claude/agents/`. Keep matching rules and reviewer guidance aligned across both directories. Use the session's model settings; reviewer roles do not select a model or change permissions.
