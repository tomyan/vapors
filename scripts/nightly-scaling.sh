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

ensure_worktree() {
  git -C "$source_checkout" fetch -q origin
  [ -e "$worktree" ] || git -C "$source_checkout" worktree add -q --detach "$worktree" "$ref"
  if [ -e "$worktree/target" ] && [ ! -L "$worktree/target" ]; then
    echo "$worktree/target is not a symlink; fix: remove it, then rerun" >&2
    return 1
  fi
  mkdir -p "$target"
  [ -L "$worktree/target" ] || ln -s "$target" "$worktree/target"
  git -C "$worktree" checkout -q --detach "$ref"
}

# The open issue's number, or nothing.
open_issue() {
  "$gh" issue list -R "$repo" --state open --search "\"$title\" in:title" \
    --json number,title --jq ".[] | select(.title == \"$title\") | .number" | head -n 1
}

# From vapors-perf's failure to the end of the log, or its last 40 lines
# when the run failed before the judge (the build, a tree with changes).
failing_lines() {
  if grep -q 'FAIL perf:' "$1"; then sed -n '/FAIL perf:/,$p' "$1"; else tail -n 40 "$1"; fi
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

failure_body() {
  echo "${3:-\`cargo xtask scaling\` failed at $2} ($(date -u +%FT%TZ))."
  echo
  commits_since_pass "$2"
  printf '\n```\n%s\n```\n\n' "$(failing_lines "$1")"
  echo "Reproduce: \`cargo xtask scaling\` in a clean checkout at $2. Log on the dev machine: $1"
}

report_failure() {
  local body number
  body=$(failure_body "$@")
  number=$(open_issue)
  if [ -n "$number" ]; then
    "$gh" issue comment "$number" -R "$repo" --body "$body"
  else
    "$gh" issue create -R "$repo" --title "$title" --body "$body"
  fi
}

report_pass() {
  local number
  echo "$1" >"$state/last-pass"
  number=$(open_issue)
  [ -z "$number" ] && return 0
  "$gh" issue close "$number" -R "$repo" \
    --comment "\`cargo xtask scaling\` passed at $1 ($(date -u +%FT%TZ)); closing."
}

# A failure of this script itself, not of the judge, reported the same way.
runner_failed() {
  trap - ERR
  local log="$state/logs/runner-$(date +%F-%H%M%S).log"
  echo "scripts/nightly-scaling.sh failed at line $1; journalctl --user -u $unit" >"$log"
  report_failure "$log" "$(git -C "$worktree" rev-parse HEAD 2>/dev/null || echo unknown)" \
    "The nightly runner failed before the judge could pass or fail"
  exit 1
}

run_once() {
  mkdir -p "$state/logs"
  trap 'runner_failed $LINENO' ERR
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
  if [ ! -x "$worktree/scripts/nightly-scaling.sh" ]; then
    echo "$ref has no scripts/nightly-scaling.sh yet; fix: install after it merges" >&2
    return 1
  fi
  local dir="$HOME/.config/systemd/user"
  mkdir -p "$dir"
  # xtask's deadlines are 30 min for the build and 20 for the judge; the
  # rest covers the fetch, compiling xtask and the issue calls.
  printf '[Unit]\nDescription=%s\n\n[Service]\nType=oneshot\nExecStart=%s\nTimeoutStartSec=90min\n' \
    "vapors nightly scaling judge (scripts/nightly-scaling.sh)" \
    "$worktree/scripts/nightly-scaling.sh" >"$dir/$unit.service"
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
