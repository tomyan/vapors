#!/usr/bin/env bash
# Proves scripts/nightly-scaling-issues.sh and the run markers against
# scripts/fixtures/nightly-scaling.log, with a fake gh: no network, no
# build. The fixture is a judge log in the harness's line format (pinned
# by crates/perf/src/scaling_all_tests.rs), its shape from the run at
# 68371b9, with a change failing for a reason other than growth (Text)
# and a second failing kind (Class, remove frames) added.
# Exits non-zero naming each check that failed.
#
# Usage: scripts/nightly-scaling-issues-check.sh
set -uo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
fixture="$here/fixtures/nightly-scaling.log"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cat >"$work/gh" <<'FAKE'
#!/usr/bin/env bash
printf '%s\n' "gh $*" >>"$FAKE_GH_LOG"
[ "$1 $2" = "issue list" ] || exit 0
prefix=$(sed -n 's/.*startswith("\([^"]*\)").*/\1/p' <<<"${@: -1}")
awk -F'\t' -v p="$prefix" 'index($2, p) == 1' "$FAKE_OPEN"
FAKE
chmod +x "$work/gh"
repo=tomyan/vapors gh="$work/gh" network_deadline=10 worktree="$here/.."
export FAKE_GH_LOG="$work/gh.log" FAKE_OPEN="$work/open"
# shellcheck source=scripts/nightly-scaling-issues.sh
source "$here/nightly-scaling-issues.sh" || { echo "FAIL sourcing nightly-scaling-issues.sh"; exit 1; }
failed=0

reset() { state="$work/state"; rm -rf "$state"; mkdir -p "$state/pass"; : >"$FAKE_GH_LOG"; : >"$FAKE_OPEN"; }
calls() { grep -c "$1" "$FAKE_GH_LOG" || true; }
check() { if [ "$2" = "$3" ]; then echo "ok   $1"; else echo "FAIL $1: expected [$3], got [$2]"; failed=1; fi; }

reset
check "changes listed" "$(expected_changes "$fixture" | xargs)" \
  "Class FlexText HoveredClass Idle Insert Resize Scroll ScrollSize Text"
check "changes failed" "$(changes_that "$fixture" fail | xargs)" "Class HoveredClass Text"
check "a block is its change's alone" "$(change_block "$fixture" Class | grep -c '74854')" 0
check "parts of a growth failure" "$(change_block "$fixture" Class | failing_parts)" \
  "add frames frame (CPU), add frames paint, add frames update, remove frames paint"

reset; report_change_failure Text "$fixture" abc
check "a failure not by growth is titled so" "$(calls 'title Nightly scaling: Text: failed, not by growth')" 1
check "and quotes its error" "$(calls 'perf_event_paranoid is 4')" 1

reset; long=$(for i in $(seq 1 40); do echo "- phase$i grows with the page"; done)
title="Nightly scaling: Class: $(failing_parts <<<"scaling: Class, add frames does work
$long")"
check "a long title is cut under 256" "$([ ${#title} -le 256 ] && echo yes)" yes

reset; printf '12\tNightly scaling: Class: add frames paint\n' >"$FAKE_OPEN"
report_change_failure Class "$fixture" abc
check "changed parts retitle the issue" "$(calls 'issue edit 12')$(calls 'Retitled; was')" 11

reset; head -n "$(grep -n 'scaling-result: Class fail' "$fixture" | cut -d: -f1)" "$fixture" >"$work/cut.log"
report_results "$work/cut.log" abc 1
check "a judge stopped partway names what it did not judge" \
  "$(calls 'no result for HoveredClass Insert Resize Scroll ScrollSize')" 1

reset; report_results "$fixture" abc 1
check "a complete run with failures files no runner issue" "$(calls "title $runner_title")" 0

reset; printf '13\tNightly scaling: Removed: paint\n' >"$FAKE_OPEN"; report_results "$fixture" abc 1
check "a change no longer judged has its issue closed" "$(calls 'issue close 13')" 1

reset; grep -v 'scaling-' "$fixture" >"$work/drift.log"; report_results "$work/drift.log" abc 0
check "a pass without result lines is a runner issue" "$(calls 'no scaling-changes line')" 1

reset; sed -e '/scaling-error-/d' -e 's/^\(scaling-result: [A-Za-z]*\) fail/\1 pass/' "$fixture" >"$work/pass.log"
echo "- killed" >"$state/UNREPORTED"
printf '14\t%s\n' "$runner_title" >"$FAKE_OPEN"
report_results "$work/pass.log" abc 0
check "an unreported run gets its own issue" "$(calls "title $unreported_title")" 1
check "and a pass closes the runner issue only" "$(calls 'issue close')$(calls 'issue close 14')" 11

# The marker functions, from the runner.
source <(sed -n '/^record_stale_marker() {/,/^}/p; /^unit_failed() {/,/^}/p' "$here/nightly-scaling.sh") ||
  { echo "FAIL sourcing the marker functions"; exit 1; }
unit=vapors-nightly-scaling
reset; echo "inv-a" >"$state/running"
MONITOR_INVOCATION_ID=inv-b unit_failed
check "OnFailure leaves another run's marker" "$([ -f "$state/UNREPORTED" ] && echo recorded)" ""
MONITOR_INVOCATION_ID=inv-a unit_failed
check "and records its own run's" "$(grep -c 'invocation inv-a' "$state/UNREPORTED")" 1
reset; echo "pid 1" >"$state/running"; record_stale_marker
check "a marker no unit recorded is recorded by the next run" "$(grep -c 'a run (pid 1)' "$state/UNREPORTED")" 1

exit "$failed"
