#!/usr/bin/env bash
set -Eeuo pipefail
umask 077

binary=${1:-/usr/bin/rustdesk}
stage=$(mktemp -d "${RUNNER_TEMP:-/tmp}/rustdesk-smoke-XXXXXXXX")
server_pid=''
cleanup() {
    result=$?
    trap - EXIT
    if [[ -n $server_pid ]]; then
        kill -TERM -- "-$server_pid" 2>/dev/null || true
        wait "$server_pid" 2>/dev/null || true
    fi
    if [[ $result != 0 ]]; then cat "$stage/server.log"; fi
    rm -rf -- "$stage"
    exit "$result"
}
trap cleanup EXIT

printf '%s' 'smoke-test-pass-123' > "$stage/password"
# Hosted runners have no logged-in desktop for the system service to start.
setsid "$binary" --server > "$stage/server.log" 2>&1 &
server_pid=$!
for attempt in $(seq 1 30); do
    if ! kill -0 "$server_pid" 2>/dev/null; then
        echo 'The installed desktop server exited before IPC became ready.' >&2
        exit 1
    fi
    if output=$(timeout 10 "$binary" --password-file "$stage/password" 2>&1); then
        if grep -Fxq 'Done!' <<< "$output"; then
            printf '%s\n' "$output"
            exit 0
        fi
        case "$output" in
            'No such file or directory (os error 2)'|'Connection refused (os error 111)'|'No --server process found for user main IPC') ;;
            *) printf '%s\n' "$output" >&2; exit 1 ;;
        esac
    else
        printf '%s\n' "$output" >&2
        exit 1
    fi
    sleep 1
done
echo 'The installed desktop server did not acknowledge the password change.' >&2
exit 1
