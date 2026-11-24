#!/usr/bin/env bash
# Install and uninstall the MSI, then check files, shortcuts and user-data preservation. Disposable CI runners only.
set -euo pipefail
source "$(dirname -- "${BASH_SOURCE[0]}")/shell-common.sh"
cd -- "$repository_root"

if [[ "${CI:-}" != "true" ]]; then
    printf 'Install/uninstall smoke tests must run in a disposable CI runner.\n' >&2
    exit 1
fi

installer="$(cygpath -w "$PWD/dist/calendar-windows-x64.msi")"
installation_directory="$(cygpath -u "$PROGRAMFILES")/Calendar"
menu_directory="$(cygpath -u "$PROGRAMDATA")/Microsoft/Windows/Start Menu/Programs/Calendar"
data_directory="$(cygpath -u "$LOCALAPPDATA")/Calendar"
preserved_file="$data_directory/installer-smoke-test.txt"
installed=false

mkdir -p "$data_directory"
printf 'Uninstall must preserve user data.\n' > "$preserved_file"

run_installer() {
    local action="$1" log_name="$2"
    MSYS2_ARG_CONV_EXCL='*' cmd.exe /c msiexec "$action" "$installer" /qn /norestart /l*v "$(cygpath -w "$PWD/dist/$log_name")"
}

cleanup() {
    if "$installed"; then run_installer /x msi-cleanup.log || true; fi
    rm -f -- "$preserved_file"
}
trap cleanup EXIT

run_installer /i msi-install.log
installed=true

for file in calendar-desktop.exe Run-Calendar.cmd .env.example; do
    [[ -f "$installation_directory/$file" ]] || { printf 'Missing installed file: %s\n' "$file" >&2; exit 1; }
done

for shortcut in Calendar.lnk "Uninstall Calendar.lnk"; do
    [[ -f "$menu_directory/$shortcut" ]] || { printf 'Missing shortcut: %s\n' "$shortcut" >&2; exit 1; }
done

run_installer /x msi-uninstall.log
installed=false

[[ ! -f "$installation_directory/calendar-desktop.exe" ]] || { printf 'Uninstall left the executable behind.\n' >&2; exit 1; }
[[ ! -f "$menu_directory/Calendar.lnk" ]] || { printf 'Uninstall left its Start menu shortcut behind.\n' >&2; exit 1; }
[[ -f "$preserved_file" ]] || { printf 'Uninstall removed user data.\n' >&2; exit 1; }

printf 'MSI install, shortcuts, uninstall, and user-data preservation passed.\n'
