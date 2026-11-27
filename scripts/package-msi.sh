#!/usr/bin/env bash
# Build dist/calendar-windows-x64.msi from the staged portable package. Usage: package-msi.sh [--version 1.2.3]
set -euo pipefail
source "$(dirname -- "${BASH_SOURCE[0]}")/shell-common.sh"
cd -- "$repository_root"

version="0.1.0"

if [[ "${1:-}" == "--version" ]]; then
    version="${2:?--version needs a value}"
fi

if ! [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    printf 'Version must be three numeric parts, for example 1.10.1.\n' >&2
    exit 1
fi

package_directory="dist/calendar-windows-x64"

if [[ ! -d "$package_directory" ]]; then
    bash "$script_directory/package-windows.sh"
fi

mkdir -p dist/msi
license_path="dist/msi/License.rtf"
{
    printf '{\\rtf1\\ansi '
    sed -e 's/\\/\\\\/g' -e 's/{/\\{/g' -e 's/}/\\}/g' -e 's/$/\\par /' LICENSE | tr -d '\n'
    printf '}'
} > "$license_path"

MSYS2_ARG_CONV_EXCL='*' wix build installer/windows/Calendar.wxs -arch x64 -ext WixToolset.UI.wixext -ext WixToolset.Util.wixext \
    -d "ProductVersion=$version" \
    -d "LicensePath=$(cygpath -w "$PWD/$license_path")" \
    -d "PackagePath=$(cygpath -w "$PWD/$package_directory")" \
    -o dist/calendar-windows-x64.msi
printf 'Created dist/calendar-windows-x64.msi (%s).\n' "$version"
