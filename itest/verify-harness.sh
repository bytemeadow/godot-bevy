#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/godot-bevy-harness.XXXXXX")"
trap 'chmod -R u+w "$TMP_ROOT" 2>/dev/null || true; rm -rf "$TMP_ROOT"' EXIT

fail() {
    echo "FAIL $*" >&2
    exit 1
}

run_report() {
    local label=$1
    local expected=$2
    local feature=$3
    local deny_focus=$4
    shift 4

    local report="$TMP_ROOT/$label.json"
    local log="$TMP_ROOT/$label.log"
    local feature_args=()
    if [[ -n $feature ]]; then
        feature_args=("$feature")
    fi

    set +e
    env ITEST_DENY_FOCUS="$deny_focus" ITEST_REPEAT=1 ITEST_TIMEOUT_FRAMES=600 \
        "$SCRIPT_DIR/run-tests.sh" ${feature_args[@]+"${feature_args[@]}"} --json "$report" "$@" \
        >"$log" 2>&1
    local status=$?
    set -e

    if [[ $status -ne $expected ]]; then
        cat "$log" >&2
        fail "$label: expected exit $expected, got $status"
    fi
    [[ -f $report ]] || {
        cat "$log" >&2
        fail "$label: report was not written"
    }
    LAST_REPORT=$report
}

assert_report() {
    if ! jq -e "$@" "$LAST_REPORT" >/dev/null; then
        jq . "$LAST_REPORT" >&2
        fail "report assertion failed: $*"
    fi
}

run_status_only() {
    local label=$1
    local expected=$2
    shift 2
    local log="$TMP_ROOT/$label.log"

    set +e
    env ITEST_DENY_FOCUS=0 ITEST_REPEAT=1 ITEST_TIMEOUT_FRAMES=600 \
        "$SCRIPT_DIR/run-tests.sh" "$@" >"$log" 2>&1
    local status=$?
    set -e
    if [[ $status -ne $expected ]]; then
        cat "$log" >&2
        fail "$label: expected exit $expected, got $status"
    fi
}

verify_repeat() {
    run_report repeat 1 --harness-probes 0 \
        --filter __harness_probe_flaky --repeat 3
    assert_report '.complete == true and .outcome == "fail" and .tests[0].outcome == "flaky"'
    assert_report '[.tests[0].attempts[].outcome] == ["pass", "fail", "pass"]'
    echo "PASS repeat: outcome=flaky attempts=pass,fail,pass exit=1"

    run_report timeout 1 --harness-probes 0 \
        --filter __harness_probe_timeout --timeout-frames 2
    assert_report '.complete == true and .outcome == "fail" and .tests[0].outcome == "fail"'
    assert_report 'any(.tests[0].attempts[0].failures[]; .kind == "timeout")'
    echo "PASS timeout: outcome=fail kind=timeout exit=1"
}

verify_panic() {
    local label filter sentinel output
    while IFS='|' read -r label filter sentinel output; do
        run_report "$label" 1 --harness-probes 0 --filter "$filter"
        assert_report --arg sentinel "$sentinel" \
            'any(.tests[0].attempts[0].failures[]; .message | contains($sentinel))'
        echo "$output"
    done <<'CASES'
sync-panic|__harness_probe_sync_panic|sync panic sentinel|PASS sync panic: sync panic sentinel
async-startup-panic|__harness_probe_async_startup_panic|async startup panic sentinel|PASS async startup panic: async startup panic sentinel
async-task-panic|__harness_probe_async_task_panic|async task panic sentinel|PASS async task panic: async task panic sentinel
bevy-frame-panic|__harness_probe_bevy_frame_panic|Bevy frame panic sentinel|PASS Bevy frame panic: Bevy frame panic sentinel
CASES
}

verify_config() {
    run_report empty-filter 2 '' 0 --filter ' , '
    assert_report '.complete == true and .outcome == "error" and .errors[0].kind == "configuration"'
    echo "PASS empty filter: exit=2"

    run_report zero-selection 2 '' 0 --filter __harness_no_such_test
    assert_report '.complete == true and .outcome == "error" and .selection.selected == 0'
    echo "PASS zero selection: exit=2"

    local invalid_report="$TMP_ROOT/invalid-repeat.json"
    run_status_only invalid-repeat 2 --json "$invalid_report" --repeat 0
    [[ ! -e $invalid_report ]] || fail "invalid repeat unexpectedly wrote a report"
    echo "PASS invalid repeat: exit=2"

    local unwritable_dir="$TMP_ROOT/unwritable"
    local unwritable_report="$unwritable_dir/report.json"
    mkdir "$unwritable_dir"
    chmod 500 "$unwritable_dir"
    run_status_only unwritable-report 2 --json "$unwritable_report" --filter test_exactly_one_clear
    chmod 700 "$unwritable_dir"
    [[ ! -e $unwritable_report ]] || fail "unwritable report path unexpectedly contains a report"
    echo "PASS unwritable report: exit=2"
}

verify_focus() {
    run_report focus 2 --harness-focus-probe 1 --filter __harness_probe_focus
    assert_report '.complete == true and .outcome == "error" and .selection.focus_run == true'
    assert_report '(.summary.attempts_passed + .summary.attempts_failed) == 0'
    echo "PASS focus denied: outcome=error focus_run=true exit=2 attempts=0"
}

if [[ $# -ne 1 ]]; then
    echo "usage: $0 repeat|panic|config|focus" >&2
    exit 2
fi

case $1 in
    repeat) verify_repeat ;;
    panic) verify_panic ;;
    config) verify_config ;;
    focus) verify_focus ;;
    *)
        echo "usage: $0 repeat|panic|config|focus" >&2
        exit 2
        ;;
esac
