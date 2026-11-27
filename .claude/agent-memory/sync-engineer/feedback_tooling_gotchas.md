---
name: tooling-gotchas
description: Shell and cargo gotchas on this Windows repo that cost time (cargo.sh cwd, rm prompts, RTK-filtered output)
metadata:
  type: feedback
---

`scripts/cargo.sh` does `cd` to the repo root, so a scratch crate must be built with `--manifest-path`; otherwise it builds the whole repo (a release desktop build took 6+ minutes).

**Why:** a probe build run from the scratchpad silently compiled the GPUI app instead.

**How to apply:** use your own `CARGO_TARGET_DIR` and `--manifest-path <scratch>/Cargo.toml` for size or memory probes. Use `rm -f` (plain `rm` prompts and hangs). Shell output from `grep`/`cat` may be filtered by RTK and truncated; use the Read tool for exact file content.

Also: a Python `subprocess.run(["bash", ...])` resolves to WSL bash and fails; use `C:/Program Files/Git/bin/bash.exe`. Bash heredocs containing triple quotes can fail to parse in the tool; write the script with the Write tool and run it with `uv run python <path>`. `-D warnings` rejects unused `pub` items in the binary crate `calendar-desktop`; use `#[expect(dead_code, reason = ...)]` (with `cfg_attr(not(test), ...)` if tests use it) for API that a later UI step will call. A `Fake::count("POST")` must skip the `/token` POSTs.
