#!/usr/bin/env bash
# Rebuild and relaunch the app whenever sources change. This is a restart, not hot reload: transient UI state resets.
set -uo pipefail
source "$(dirname -- "${BASH_SOURCE[0]}")/shell-common.sh"
cd -- "$repository_root"

case "${1:-}" in
    --once) exec bash "$script_directory/start.sh" ;;
    "") ;;
    *) printf 'Usage: bash scripts/dev-desktop.sh [--once]\n' >&2; exit 1 ;;
esac

printf 'Watching crates, assets and manifests. Ctrl+C stops watching; the app stays open.\n'

while true; do
    # Snapshot before building so a save during the build triggers another round.
    snapshot_before_build="$(source_snapshot "$repository_root")"
    bash "$script_directory/start.sh" || printf 'Build failed; the running app was kept. Fix the error and save again.\n' >&2

    while [[ "$(source_snapshot "$repository_root")" == "$snapshot_before_build" ]]; do
        sleep 1
    done

    # Wait for a quiet second before rebuilding a batch of saved files.
    while true; do
        quiet_snapshot="$(source_snapshot "$repository_root")"
        sleep 1
        if [[ "$(source_snapshot "$repository_root")" == "$quiet_snapshot" ]]; then break; fi
    done
done
