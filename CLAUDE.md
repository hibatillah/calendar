# Calendar

Windows-first GPUI calendar with an Axum/GraphQL backend. Preserve macOS boundaries; mobile and web are deferred.

- Before changing code, read [CONTRIBUTING.md](CONTRIBUTING.md) for module ownership, naming, testing and logging conventions.
- For cross-layer changes, read [docs/architecture.md](docs/architecture.md). For Google integration, also read [docs/google.md](docs/google.md).
- Use [README.md](README.md) for current build/run commands. Executable workflows belong in `scripts/`.
- Project instructions live in `.claude/rules/`; path-scoped rules apply to their matching files. Treat them as desired behavior, not proof that the current implementation satisfies it.
- For desktop interaction reviews, use `desktop-interaction-reviewer`. For account, sync or persistence reviews, use `calendar-sync-reviewer`. Supply the changed paths and available test evidence. These agents inspect and report; the main agent owns fixes, test execution and runtime verification.
- Keep implementation status in the existing docs. Avoid copying changing status or secret values into instructions.

