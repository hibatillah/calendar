# Project workflow

- Follow the explicit identifier and module ownership conventions in `CONTRIBUTING.md`; use names such as `calendar_id` and `operation_id`, never cryptic abbreviations.
- Keep work local. Commit, push, publish or deploy only when the user explicitly requests it. Continue authorized implementation without repeated permission questions.
- After the initial bootstrap commit, use a feature branch and PR for every change; never push changes directly to `main`. Wait for CI to pass before any authorized merge. Opening a PR does not authorize merging it.
- On Windows, use `./scripts/cargo.ps1` so the MSVC linker and Windows SDK are selected correctly. Run Python through `uv run`, including one-liners.
- For web/mobile packages, use Bun workspaces and a single `bun.lock`. Use Turborepo for TypeScript task coordination; keep Rust builds under Cargo. See `docs/architecture.md` for the tooling boundaries.
- Use `./scripts/start.ps1` when a rebuild/relaunch is part of the task; it stops this repo's running app. Review-only and documentation-only work should leave it running.
- Keep calendar domain and layout crates independent of GPUI, HTTP and storage. Keep native OS code behind target gates; use the background service for network/database work.
- When `.codegraph/` exists, use CodeGraph before text searches to locate or understand code. Without an index, use targeted searches; creating an index is the user's decision.
- For GitHub work, prefer available GitHub MCP tools; use `gh` for capabilities they lack. Honor the user's requested scope.
