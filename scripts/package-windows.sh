#!/usr/bin/env bash
# Build the release executable and stage the portable ZIP in dist/.
set -euo pipefail
source "$(dirname -- "${BASH_SOURCE[0]}")/shell-common.sh"
cd -- "$repository_root"

package_directory="dist/calendar-windows-x64"

if [[ -e "$package_directory" ]]; then
    printf 'Package directory already exists. Remove dist/ before packaging.\n' >&2
    exit 1
fi

cargo_command build --locked --release -p calendar-desktop

mkdir -p "$package_directory/third-party-licenses"
cp -- target/release/calendar-desktop.exe "$package_directory/"
cp -- installer/windows/Run-Calendar.cmd .env.example LICENSE README.md docs/google.md "$package_directory/"
cp -- assets/fonts/OFL.txt "$package_directory/third-party-licenses/"

rm -f dist/calendar-windows-x64.zip
powershell.exe -NoProfile -Command "Compress-Archive -Force -Path '$(cygpath -w "$PWD/$package_directory")\\*' -DestinationPath '$(cygpath -w "$PWD/dist/calendar-windows-x64.zip")'"
printf 'Created dist/calendar-windows-x64.zip (no local state or credentials included).\n'
