#!/usr/bin/env bash
set -euo pipefail
source "$(dirname -- "${BASH_SOURCE[0]}")/shell-common.sh"

once=false

case "${1:-}" in
    --once) once=true; shift ;;
esac

if [[ $# -ne 0 ]]; then
    printf 'Usage: bash scripts/dev-desktop.sh [--once]\n' >&2
    exit 1
fi

if is_windows; then
    if "$once"; then
        run_windows_helper dev-desktop -Once
    else
        run_windows_helper dev-desktop
    fi
    exit
fi

cd -- "$repository_root"
mkdir -p .local
lock_directory="$repository_root/.local/bash-desktop.lock"

if ! mkdir "$lock_directory" 2>/dev/null; then
    printf 'Another Bash desktop session is running. If it crashed, remove %s after checking its processes.\n' "$lock_directory" >&2
    exit 1
fi

desktop_process=''
desktop_checksum=''

cleanup() {
    for process_id in "$desktop_process"; do
        if [[ -n "$process_id" ]]; then
            kill "$process_id" 2>/dev/null || true
            wait "$process_id" 2>/dev/null || true
        fi
    done

    rmdir -- "$lock_directory"
}

trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

printf 'Watching Rust sources and assets. Ctrl+C stops this session and its applications.\n'
printf 'Rebuilds restart the changed application; unsaved editor state resets.\n'

while true; do
    snapshot_before_build="$(source_snapshot "$repository_root")"
    build_status=0
    bash "$script_directory/cargo.sh" build --workspace || build_status=$?

    if [[ "$build_status" -eq 0 ]]; then
        binary="$repository_root/target/debug/calendar-desktop"
        checksum="$(cksum < "$binary")"

        if [[ "$checksum" != "$desktop_checksum" || -z "$desktop_process" ]] || ! kill -0 "$desktop_process" 2>/dev/null; then
            runtime_directory="$(mktemp -d "$repository_root/.local/bash-desktop.XXXXXX")"
            cp -- "$binary" "$runtime_directory/calendar-desktop"

            if [[ -n "$desktop_process" ]]; then
                kill "$desktop_process" 2>/dev/null || true
                wait "$desktop_process" 2>/dev/null || true
            fi

            "$runtime_directory/calendar-desktop" &
            desktop_process="$!"
            desktop_checksum="$checksum"
        fi
    else
        printf 'Build failed; running applications were kept open.\n' >&2
    fi

    if "$once"; then
        if [[ "$build_status" -ne 0 ]]; then exit "$build_status"; fi
        wait "$desktop_process"
        exit
    fi

    while [[ "$(source_snapshot "$repository_root")" == "$snapshot_before_build" ]]; do
        sleep 1
    done

    # Wait for a quiet interval before rebuilding a batch of saved files.
    while true; do
        snapshot_before_build="$(source_snapshot "$repository_root")"
        sleep 1
        if [[ "$(source_snapshot "$repository_root")" == "$snapshot_before_build" ]]; then break; fi
    done
done
