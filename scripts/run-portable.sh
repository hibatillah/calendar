#!/usr/bin/env bash
set -euo pipefail

case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*) ;;
    *) printf 'This portable package contains Windows executables. Use it in Git Bash on Windows.\n' >&2; exit 1 ;;
esac

package_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
launcher="$package_directory/Run-Calendar.ps1"

if [[ ! -f "$launcher" ]]; then
    printf 'Run this launcher from the extracted Windows portable package.\n' >&2
    exit 1
fi

exec powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$(cygpath -w "$launcher")"
