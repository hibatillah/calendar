#!/usr/bin/env bash
# Build the debug app and (re)launch the tray and UI executables from a private copy, so the build output is never locked by a running .exe.
set -euo pipefail
source "$(dirname -- "${BASH_SOURCE[0]}")/shell-common.sh"
cd -- "$repository_root"

cargo_command build -p calendar-desktop -p calendar-tray

run_directory="$repository_root/.local/run"
mkdir -p "$run_directory"

if is_windows; then
    # The tray starts the UI as the sibling calendar-desktop.exe, so both are copied side by side.
    launch_directory="$run_directory/run-$(date +%H%M%S-%N | cut -c1-10)"

    # Stop only copies started from this checkout's run directory.
    windows_run_directory="$(cygpath -w "$run_directory")"
    powershell.exe -NoProfile -Command "Get-Process calendar-tray,calendar-desktop -ErrorAction SilentlyContinue | Where-Object { \$_.Path -like '$windows_run_directory\\*' } | Stop-Process -Force" || true
    sleep 1
    rm -rf -- "$run_directory"/run-* "$run_directory"/calendar-desktop-*.exe 2>/dev/null || true
    mkdir -p "$launch_directory"
    cp -- target/debug/calendar-desktop.exe target/debug/calendar-tray.exe "$launch_directory/"
    # The tray keeps running after the window closes; the UI process ends with its window.
    cmd.exe //c start "" "$(cygpath -w "$launch_directory/calendar-tray.exe")" > /dev/null 2>&1 < /dev/null
    cmd.exe //c start "" "$(cygpath -w "$launch_directory/calendar-desktop.exe")" > /dev/null 2>&1 < /dev/null
    launched_executable="$launch_directory"
else
    launched_executable="$run_directory"
    pkill -f -- "$run_directory/calendar-" 2>/dev/null || true
    cp -- target/debug/calendar-desktop target/debug/calendar-tray "$run_directory/"
    "$run_directory/calendar-tray" &
    "$run_directory/calendar-desktop" &
fi

printf 'running: %s\n' "$launched_executable"
