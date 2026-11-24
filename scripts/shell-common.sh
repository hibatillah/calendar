#!/usr/bin/env bash

script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repository_root="$(cd -- "$script_directory/.." && pwd)"

is_windows() {
    case "$(uname -s)" in
        MINGW*|MSYS*|CYGWIN*) return 0 ;;
        *) return 1 ;;
    esac
}

# GPUI builds only with the MSVC host toolchain. Name it explicitly so Git's link.exe is never picked.
cargo_command() {
    if is_windows; then
        cargo +stable-x86_64-pc-windows-msvc "$@"
    else
        cargo "$@"
    fi
}

source_snapshot() {
    local root="$1"

    (
        cd -- "$root"

        for source_path in crates assets .cargo; do
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
