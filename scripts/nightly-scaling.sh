#!/usr/bin/env bash
# The nightly scaling judge: `cargo xtask scaling` on origin/main, in its
# own worktree on this machine, which has the GPU (GitHub's runners have
# none). A failure opens one GitHub issue titled "Nightly scaling judge
# failing on main", or comments on it when it is open, with the failing
# changes, the revision and the commits since the last passing revision
# (kept in $state/last-pass). A pass records the revision and closes the
# issue with a comment. Exits with the judge's status.
#
# Usage: scripts/nightly-scaling.sh            one run now
#        scripts/nightly-scaling.sh --install  the worktree and the systemd
#                                              --user timer (03:00), not enabled
# Overrides, for trying it: NIGHTLY_REF (origin/main), NIGHTLY_GH (gh),
# NIGHTLY_STATE (~/.cache/nightly-scaling).
#
# Not covered yet (the follow-up PR: one issue per change, and an
# OnFailure= unit's local marker): a run killed at the unit's timeout, or
# one whose report gh could not file, is only in
# `journalctl --user -u vapors-nightly-scaling`. The timer stays off
# until that PR merges.
set -eEuo pipefail
# Hook variables and the session bus are never passed on (CLAUDE.md).
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE DBUS_SESSION_BUS_ADDRESS
export PATH="$HOME/.cargo/bin:$PATH"

repo=tomyan/vapors
title="Nightly scaling judge failing on main"
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

github() { timeout "$network_deadline" "$gh" "$@"; }

# The open issue's number, or nothing.
open_issue() {
  github issue list -R "$repo" --state open --search "\"$title\" in:title" \
    --json number,title --jq ".[] | select(.title == \"$title\") | .number" | head -n 1
}

# Comments on the open issue, or opens it, with body $1.
file_issue() {
  local number
  number=$(open_issue)
  if [ -n "$number" ]; then
    github issue comment "$number" -R "$repo" --body "$1"
  else
    github issue create -R "$repo" --title "$title" --body "$1"
  fi
}

# From vapors-perf's failure to the end of the log, or its last 40 lines
# when the run failed before the judge; under GitHub's 65,536 characters.
failing_lines() {
  { if grep -q 'FAIL perf:' "$1"; then sed -n '/FAIL perf:/,$p' "$1"; else tail -n 40 "$1"; fi; } | head -c 50000
}

commits_since_pass() {
  if [ ! -f "$state/last-pass" ]; then
    echo "No passing revision is recorded ($state/last-pass), so the range is unknown."
    return 0
  fi
  local since
  since=$(cat "$state/last-pass")
  echo "Since the last passing revision: https://github.com/$repo/compare/$since...$1"
  echo
  git -C "$worktree" log --format='- %h %s' "$since..$1" | head -n 50
}

report_failure() {
  file_issue "\`cargo xtask scaling\` failed at $2 ($(date -u +%FT%TZ)).

$(commits_since_pass "$2")

\`\`\`
$(failing_lines "$1")
\`\`\`

Reproduce: \`cargo xtask scaling\` in a clean checkout at $2. Log on the dev machine: $1"
}

report_pass() {
  local number
  echo "$1" >"$state/last-pass"
  number=$(open_issue)
  [ -z "$number" ] && return 0
  github issue close "$number" -R "$repo" \
    --comment "\`cargo xtask scaling\` passed at $1 ($(date -u +%FT%TZ)); closing."
}

# A failure of this script itself, not of the judge; its stderr is in
# $runner_log. In a command substitution's subshell the trap fires too:
# that one exits, and the parent, whose assignment then fails, reports.
runner_failed() {
  trap - ERR
  [ "$BASH_SUBSHELL" -gt 0 ] && exit 1
  file_issue "The nightly runner failed before the judge could pass or fail ($(date -u +%FT%TZ)): \`$2\` at line $1 of scripts/nightly-scaling.sh.

\`\`\`
$(tail -n 40 "$runner_log")
\`\`\`

Reproduce: \`scripts/nightly-scaling.sh\` on the dev machine. Log: $runner_log"
  exit 1
}

run_once() {
  mkdir -p "$state/logs"
  exec 9>"$state/lock"
  flock -n 9 || { echo "another nightly run holds $state/lock; fix: wait for it to end" >&2; exit 1; }
  runner_log="$state/logs/$(date +%F-%H%M%S)-runner.log"
  exec 2> >(tee -a "$runner_log" >&2)
  trap 'runner_failed $LINENO "$BASH_COMMAND"' ERR
  ensure_worktree
  local revision log
  revision=$(git -C "$worktree" rev-parse HEAD)
  log="$state/logs/$(date +%F)-$revision.log"
  (cd "$worktree" && nice -n 10 cargo xtask scaling) 2>&1 | tee "$log" || judge_status=$?
  trap - ERR
  if [ "$judge_status" -eq 0 ]; then report_pass "$revision"; else report_failure "$log" "$revision"; fi
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
  printf '[Unit]\nDescription=%s\n\n[Service]\nType=oneshot\nExecStart=%s\nTimeoutStartSec=90min\n' \
    "vapors nightly scaling judge (scripts/nightly-scaling.sh)" "$script" >"$dir/$unit.service"
  printf '[Unit]\nDescription=%s\n\n[Timer]\nOnCalendar=*-*-* 03:00:00\nPersistent=true\n\n[Install]\nWantedBy=timers.target\n' \
    "$unit.service at 03:00" >"$dir/$unit.timer"
  systemctl --user daemon-reload
  echo "installed $dir/$unit.service and .timer, not enabled; enable: systemctl --user enable --now $unit.timer"
}

# The run checks out a new revision of this file: main runs from the
# definitions already read, and the `exit` on its line stops bash
# reading the file again.
main() {
  if [ "${1:-}" = --install ]; then install_units; exit 0; fi
  run_once
  exit "$judge_status"
}
main "$@"; exit
