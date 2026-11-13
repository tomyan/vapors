#!/usr/bin/env bash
# The nightly scaling judge: `cargo xtask scaling` on origin/main, in its
# own worktree on this machine, which has the GPU (GitHub's runners have
# none). Each failing change opens its own GitHub issue, or comments on
# it, with its failure and the commits since that change last passed
# ($state/pass/<change>); a pass closes it (scripts/nightly-scaling-issues.sh).
# Exits with the judge's status.
#
# Usage: scripts/nightly-scaling.sh            one run now
#        scripts/nightly-scaling.sh --install  the worktree and the systemd
#                                              --user timer (03:00), not enabled
#        scripts/nightly-scaling.sh --unit-failed  the OnFailure= unit: no network
# Overrides, for trying it: NIGHTLY_REF (origin/main), NIGHTLY_GH (gh),
# NIGHTLY_STATE (~/.cache/nightly-scaling).
#
# A run that ends without reporting (killed at the unit's timeout, gh
# unreachable) leaves $state/running; the unit's OnFailure= unit then
# appends a line to $state/UNREPORTED, without the network, and the next
# run files it. Not covered: the worktree removed, so neither unit's
# script exists; that is only in `journalctl --user -u vapors-nightly-scaling`.
set -eEuo pipefail
# Hook variables and the session bus are never passed on (CLAUDE.md).
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE DBUS_SESSION_BUS_ADDRESS
export PATH="$HOME/.cargo/bin:$PATH"

repo=tomyan/vapors
source_checkout="$HOME/projects/vapors"
worktree="$HOME/.cache/w-nightly/vapors"
target="$HOME/.cache/vapors-targets/w-nightly"
ref=${NIGHTLY_REF:-origin/main}
gh=${NIGHTLY_GH:-gh}
state=${NIGHTLY_STATE:-$HOME/.cache/nightly-scaling}
unit=vapors-nightly-scaling
judge_status=0
# A fetch and a gh call each took 0.5 s here (2026-11-13); a minute is
# far past a slow network, and well inside the unit's timeout.
network_deadline=60
# shellcheck source=scripts/nightly-scaling-issues.sh
source "$(dirname "$0")/nightly-scaling-issues.sh"

ensure_worktree() {
  timeout "$network_deadline" git -C "$source_checkout" fetch -q origin
  [ -e "$worktree" ] || git -C "$source_checkout" worktree add -q --detach "$worktree" "$ref"
  if [ -e "$worktree/target" ] && [ ! -L "$worktree/target" ]; then
    echo "$worktree/target is not a symlink; fix: remove it, then rerun" >&2
    return 1
  fi
  mkdir -p "$target"
  [ -L "$worktree/target" ] || ln -s "$target" "$worktree/target"
  git -C "$worktree" checkout -q --detach "$ref"
}

# A failure of this script itself, not of the judge; its stderr is in
# $runner_log. In a command substitution's subshell the trap fires too:
# that one exits, and the parent, whose assignment then fails, reports.
runner_failed() {
  trap - ERR
  [ "$BASH_SUBSHELL" -gt 0 ] && exit 1
  file_issue "$runner_title" "$runner_title" "The nightly runner failed before the judge could pass or fail ($(date -u +%FT%TZ)): \`$2\` at line $1 of scripts/nightly-scaling.sh.

\`\`\`
$(tail -n 40 "$runner_log")
\`\`\`

Reproduce: \`scripts/nightly-scaling.sh\` on the dev machine. Log: $runner_log"
  rm -f "$state/running"
  exit 1
}

run_once() {
  mkdir -p "$state/logs" "$state/pass"
  exec 9>"$state/lock"
  flock -n 9 || { echo "another nightly run holds $state/lock; fix: wait for it to end" >&2; exit 1; }
  runner_log="$state/logs/$(date +%F-%H%M%S)-runner.log"
  exec 2> >(tee -a "$runner_log" >&2)
  touch "$state/running"
  trap 'runner_failed $LINENO "$BASH_COMMAND"' ERR
  ensure_worktree
  local revision log change failing
  revision=$(git -C "$worktree" rev-parse HEAD)
  log="$state/logs/$(date +%F)-$revision.log"
  (cd "$worktree" && nice -n 10 cargo xtask scaling) 2>&1 | tee "$log" || judge_status=$?
  report_unreported
  for change in $(changes_that "$log" pass); do report_change_pass "$change" "$revision"; done
  failing=$(changes_that "$log" fail)
  for change in $failing; do report_change_failure "$change" "$log" "$revision"; done
  if [ "$judge_status" -eq 0 ]; then
    close_issue "$runner_title" "\`cargo xtask scaling\` passed at $revision; closing."
  elif [ -z "$failing" ]; then
    report_run_failure "$log" "$revision"
  fi
  trap - ERR
  rm -f "$state/running"
}

# The OnFailure= unit: a run that left $state/running did not report.
unit_failed() {
  [ -e "$state/running" ] || return 0
  printf -- '- %s: result %s, exit %s %s (invocation %s); journalctl --user -u %s\n' \
    "$(date -u +%FT%TZ)" "${MONITOR_SERVICE_RESULT:-?}" "${MONITOR_EXIT_CODE:-?}" \
    "${MONITOR_EXIT_STATUS:-?}" "${MONITOR_INVOCATION_ID:-?}" "$unit" >>"$state/UNREPORTED"
  rm -f "$state/running"
}

install_units() {
  ensure_worktree
  local script="$worktree/scripts/nightly-scaling.sh" dir="$HOME/.config/systemd/user"
  if [ ! -x "$script" ]; then
    echo "$ref has no scripts/nightly-scaling.sh yet; fix: install after it merges" >&2
    return 1
  fi
  mkdir -p "$dir"
  # xtask's deadlines are 30 min for the build and 20 for the judge; the
  # rest covers the fetch, compiling xtask and the issue calls.
  printf '[Unit]\nDescription=%s\nOnFailure=%s-failed.service\n\n[Service]\nType=oneshot\nExecStart=%s\nTimeoutStartSec=90min\n' \
    "vapors nightly scaling judge (scripts/nightly-scaling.sh)" "$unit" "$script" >"$dir/$unit.service"
  printf '[Unit]\nDescription=%s\n\n[Service]\nType=oneshot\nExecStart=%s --unit-failed\n' \
    "records a $unit run that ended without reporting" "$script" >"$dir/$unit-failed.service"
  printf '[Unit]\nDescription=%s\n\n[Timer]\nOnCalendar=*-*-* 03:00:00\nPersistent=true\n\n[Install]\nWantedBy=timers.target\n' \
    "$unit.service at 03:00" >"$dir/$unit.timer"
  systemctl --user daemon-reload
  echo "installed $dir/$unit.service and .timer, not enabled; enable: systemctl --user enable --now $unit.timer"
}

# The run checks out a new revision of this file: main runs from the
# definitions already read, and the `exit` on its line stops bash
# reading the file again.
main() {
  case "${1:-}" in
    --install) install_units; exit 0 ;;
    --unit-failed) mkdir -p "$state"; unit_failed; exit 0 ;;
  esac
  run_once
  exit "$judge_status"
}
main "$@"; exit
