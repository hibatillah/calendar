#!/usr/bin/env bash
set -euo pipefail
source "$(dirname -- "${BASH_SOURCE[0]}")/shell-common.sh"
run_windows_helper package-windows "$@"
