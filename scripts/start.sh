#!/usr/bin/env bash
# Build the debug app and (re)launch it from a private copy, so the build output is never locked by a running .exe.
set -euo pipefail
source "$(dirname -- "${BASH_SOURCE[0]}")/shell-common.sh"
cd -- "$repository_root"

cargo_command build -p calendar-desktop

run_directory="$repository_root/.local/run"
mkdir -p "$run_directory"

if is_windows; then
    built_executable="target/debug/calendar-desktop.exe"
    launched_executable="$run_directory/calendar-desktop-$(date +%H%M%S-%N | cut -c1-10).exe"

    # Stop only copies started from this checkout's run directory.
    windows_run_directory="$(cygpath -w "$run_directory")"
    powershell.exe -NoProfile -Command "Get-Process calendar-desktop-* -ErrorAction SilentlyContinue | Where-Object { \$_.Path -like '$windows_run_directory\\*' } | Stop-Process -Force" || true
    rm -f -- "$run_directory"/calendar-desktop-*.exe 2>/dev/null || true
    cp -- "$built_executable" "$launched_executable"
    cmd.exe //c start "" "$(cygpath -w "$launched_executable")" > /dev/null 2>&1 < /dev/null
else
    launched_executable="$run_directory/calendar-desktop"
    pkill -f -- "$launched_executable" 2>/dev/null || true
    cp -- target/debug/calendar-desktop "$launched_executable"
    "$launched_executable" &
fi

printf 'running: %s\n' "$launched_executable"
