#!/usr/bin/env bash
set -euo pipefail
source "$(dirname -- "${BASH_SOURCE[0]}")/shell-common.sh"

test_root="$(mktemp -d)"
trap 'rm -f -- "$test_root/apps/desktop/src/main.rs" "$test_root/.local/cache"; rmdir -- "$test_root/apps/desktop/src" "$test_root/apps/desktop" "$test_root/apps" "$test_root/.local" "$test_root"' EXIT
mkdir -p "$test_root/apps/desktop/src" "$test_root/.local"
original_snapshot="$(source_snapshot "$test_root")"
printf 'ignored' > "$test_root/.local/cache"
[[ "$(source_snapshot "$test_root")" == "$original_snapshot" ]]
printf 'fn main() {}' > "$test_root/apps/desktop/src/main.rs"
added_snapshot="$(source_snapshot "$test_root")"
[[ "$added_snapshot" != "$original_snapshot" ]]
printf 'fn main() { println!("changed"); }' > "$test_root/apps/desktop/src/main.rs"
[[ "$(source_snapshot "$test_root")" != "$added_snapshot" ]]
rm -- "$test_root/apps/desktop/src/main.rs"
[[ "$(source_snapshot "$test_root")" == "$original_snapshot" ]]
printf 'Bash watcher snapshot checks passed.\n'

if is_windows; then
    run_windows_helper test-dev-desktop
fi
