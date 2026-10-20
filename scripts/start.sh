#!/usr/bin/env bash
set -euo pipefail
source "$(dirname -- "${BASH_SOURCE[0]}")/shell-common.sh"

if is_windows; then
    run_windows_helper start "$@"
else
    exec bash "$script_directory/dev-desktop.sh" --once "$@"
fi
