#!/usr/bin/env bash

script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repository_root="$(cd -- "$script_directory/.." && pwd)"

is_windows() {
    case "$(uname -s)" in
        MINGW*|MSYS*|CYGWIN*) return 0 ;;
        *) return 1 ;;
    esac
}

run_windows_helper() {
    local helper_name="$1"
    shift

    if ! is_windows; then
        printf '%s requires Windows and Git Bash; WSL is a Linux environment.\n' "$helper_name" >&2
        return 1
    fi

    if ! command -v pwsh >/dev/null 2>&1; then
        printf 'Install PowerShell 7 for Windows build tooling (pwsh must be on PATH).\n' >&2
        return 1
    fi

    pwsh -NoProfile -File "$(cygpath -w "$script_directory/windows/$helper_name.ps1")" "$@"
}

source_snapshot() {
    local root="$1"

    (
        cd -- "$root"

        for source_path in apps/desktop apps/backend crates assets .cargo; do
            if [[ -d "$source_path" ]]; then
                find "$source_path" -type f -exec cksum {} +
            fi
        done

        for manifest in Cargo.toml Cargo.lock; do
            if [[ -f "$manifest" ]]; then
                cksum "$manifest"
            fi
        done
    ) | LC_ALL=C sort | cksum
}
