#!/usr/bin/env bash
set -euo pipefail
source "$(dirname -- "${BASH_SOURCE[0]}")/shell-common.sh"

# Test the Unix orchestration without starting a real server or reading user state.
test_root="$(mktemp -d "${TMPDIR:-/tmp}/calendar-shell-test.XXXXXX")"

cleanup() {
    case "$test_root" in
        "${TMPDIR:-/tmp}"/calendar-shell-test.*) rm -rf -- "$test_root" ;;
        *) printf 'Unexpected test directory; refusing cleanup.\n' >&2 ;;
    esac
}

trap cleanup EXIT
mkdir -p "$test_root/scripts" "$test_root/bin" "$test_root/target/debug"
cp "$script_directory/"{shell-common,cargo,dev-desktop}.sh "$test_root/scripts/"

printf '#!/usr/bin/env bash\nprintf "Linux\\n"\n' > "$test_root/bin/uname"
printf '#!/usr/bin/env bash\nexit 0\n' > "$test_root/bin/cargo"
printf '#!/usr/bin/env bash\nexit "${MOCK_CURL_STATUS:-1}"\n' > "$test_root/bin/curl"
printf '#!/usr/bin/env bash\nexit 1\n' > "$test_root/target/debug/calendar-backend"
printf '#!/usr/bin/env bash\ntouch "$TEST_DESKTOP_MARKER"\n' > "$test_root/target/debug/calendar-desktop"
chmod +x "$test_root/bin/"* "$test_root/target/debug/"*
export PATH="$test_root/bin:$PATH"
export TEST_DESKTOP_MARKER="$test_root/desktop-started"

if MOCK_CURL_STATUS=0 bash "$test_root/scripts/dev-desktop.sh" --once > "$test_root/collision.log" 2>&1; then
    printf 'An occupied backend port was incorrectly accepted.\n' >&2
    exit 1
fi

[[ "$(cat "$test_root/collision.log")" == *'Port 47821 already has a service'* ]]
[[ ! -f "$TEST_DESKTOP_MARKER" ]]

if MOCK_CURL_STATUS=1 bash "$test_root/scripts/dev-desktop.sh" --once > "$test_root/startup.log" 2>&1; then
    printf 'A failed backend startup was incorrectly accepted.\n' >&2
    exit 1
fi

[[ "$(cat "$test_root/startup.log")" == *'backend exited during startup'* ]]
[[ ! -f "$TEST_DESKTOP_MARKER" ]]
printf 'Port collision and backend startup failure checks passed.\n'
