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
command -v curl >/dev/null || { printf 'Install curl for backend readiness checks.\n' >&2; exit 1; }

if curl --silent --output /dev/null --max-time 1 http://127.0.0.1:47821/health; then
    printf 'Port 47821 already has a service. Close the existing backend before starting this session.\n' >&2
    exit 1
fi

mkdir -p .local
lock_directory="$repository_root/.local/bash-desktop.lock"

if ! mkdir "$lock_directory" 2>/dev/null; then
    printf 'Another Bash desktop session is running. If it crashed, remove %s after checking its processes.\n' "$lock_directory" >&2
    exit 1
fi

backend_process=''
desktop_process=''
backend_checksum=''
desktop_checksum=''

cleanup() {
    for process_id in "$desktop_process" "$backend_process"; do
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
printf 'Rebuilds restart changed applications; unsaved editor state resets.\n'

while true; do
    snapshot_before_build="$(source_snapshot "$repository_root")"
    build_status=0
    bash "$script_directory/cargo.sh" build --workspace || build_status=$?

    if [[ "$build_status" -eq 0 ]]; then
        for application in backend desktop; do
            binary="$repository_root/target/debug/calendar-$application"
            checksum="$(cksum < "$binary")"
            checksum_variable="${application}_checksum"
            process_variable="${application}_process"
            previous_process="${!process_variable}"

            if [[ "$checksum" != "${!checksum_variable}" || -z "$previous_process" ]] || ! kill -0 "$previous_process" 2>/dev/null; then
                runtime_directory="$(mktemp -d "$repository_root/.local/bash-desktop.XXXXXX")"
                cp -- "$binary" "$runtime_directory/calendar-$application"

                if [[ -n "$previous_process" ]]; then
                    kill "$previous_process" 2>/dev/null || true
                    wait "$previous_process" 2>/dev/null || true
                fi

                "$runtime_directory/calendar-$application" &
                printf -v "$process_variable" '%s' "$!"
                printf -v "$checksum_variable" '%s' "$checksum"

                if [[ "$application" == backend ]]; then
                    backend_ready=false

                    for attempt in {1..50}; do
                        sleep 0.1

                        if ! kill -0 "$backend_process" 2>/dev/null; then
                            printf 'The new backend exited during startup; the desktop was not restarted.\n' >&2
                            exit 1
                        fi

                        if curl --fail --silent --output /dev/null --max-time 1 http://127.0.0.1:47821/health; then
                            backend_ready=true
                            break
                        fi
                    done

                    if ! "$backend_ready"; then
                        printf 'The new backend did not become ready; the desktop was not restarted.\n' >&2
                        exit 1
                    fi
                fi
            fi
        done
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
