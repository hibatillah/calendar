#!/usr/bin/env bash
set -euo pipefail
source "$(dirname -- "${BASH_SOURCE[0]}")/shell-common.sh"
cd -- "$repository_root"

if is_windows; then
    # Select MSVC before Git's unrelated link.exe.
    run_windows_helper cargo "$@"
else
    exec cargo "$@"
fi
