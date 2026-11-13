# Sourced by scripts/nightly-scaling.sh: the GitHub issues it files.
# One issue per failing change, titled "Nightly scaling: <change>: <kind>
# <phase>, ..." when it opens and found again by "Nightly scaling:
# <change>:"; it closes when that change passes. One more, titled
# $runner_title, for a run that failed before the judge named a change
# (the runner, the build, a deadline); it closes when every change passes.

runner_title="Nightly scaling judge failing on main"

github() { timeout "$network_deadline" "$gh" "$@"; }

# The open issue whose title starts with $1, or nothing.
open_issue() {
  github issue list -R "$repo" --state open --search "\"$1\" in:title" --limit 100 \
    --json number,title --jq ".[] | select(.title | startswith(\"$1\")) | .number" | head -n 1
}

# Comments on the open issue whose title starts with $1, or opens one
# titled $2, with body $3.
file_issue() {
  local number
  number=$(open_issue "$1")
  if [ -n "$number" ]; then
    github issue comment "$number" -R "$repo" --body "$3"
  else
    github issue create -R "$repo" --title "$2" --body "$3"
  fi
}

# Closes the open issue whose title starts with $1, if any, with comment $2.
close_issue() {
  local number
  number=$(open_issue "$1")
  [ -z "$number" ] && return 0
  github issue close "$number" -R "$repo" --comment "$2"
}

# The changes whose `scaling-result:` line (vapors-perf) in log $1 says $2.
changes_that() { { grep -oE "scaling-result: [A-Za-z]+ $2" "$1" || true; } | cut -d' ' -f2 | sort -u; }

# Change $2's failure in log $1: each "scaling: $2[, <kind>] does work"
# through its "reproduce:" line.
change_failure() {
  sed 's/^ *//' "$1" | awk -v change="$2" '
    $0 ~ "^scaling: " change "(,| does work)" { on = 1 }
    on { print }
    on && /^reproduce:/ { on = 0 }' | head -c 50000
}

# "<kind> <phase>, ..." of a change's failure lines on stdin.
failing_parts() {
  awk '/^scaling: / { kind = $0; sub(/^scaling: [A-Za-z]+,? ?/, "", kind); sub(/ ?does work.*/, "", kind) }
       /^- / { phase = $0; sub(/^- /, "", phase); sub(/ (grows|costs) .*/, "", phase)
               print (kind == "" ? "" : kind " ") phase }' | sort -u | paste -sd, - | sed 's/,/, /g'
}

# The commits from change $1's last pass to revision $2.
since_last_pass() {
  local since
  since=$(cat "$state/pass/$1" 2>/dev/null || true)
  if [ -z "$since" ]; then
    echo "No pass of $1 is recorded ($state/pass/$1), so the range is unknown."
    return 0
  fi
  printf 'Last passed at %s: https://github.com/%s/compare/%s...%s\n\n' "$since" "$repo" "$since" "$2"
  git -C "$worktree" log --format='- %h %s' "$since..$2" | head -n 50
}

# Change $1 failed in log $2 at revision $3.
report_change_failure() {
  local lines
  lines=$(change_failure "$2" "$1")
  file_issue "Nightly scaling: $1:" "Nightly scaling: $1: $(failing_parts <<<"$lines")" \
    "\`--scaling $1\` failed at $3 ($(date -u +%FT%TZ)).

$(since_last_pass "$1" "$3")

\`\`\`
$lines
\`\`\`

Reproduce: \`cargo build --release -p vapors-perf && target/release/vapors-perf 60 10 --scaling $1\` in a clean checkout at $3. Log on the dev machine: $2"
}

# Change $1 passed at revision $2.
report_change_pass() {
  echo "$2" >"$state/pass/$1"
  close_issue "Nightly scaling: $1:" "\`--scaling $1\` passed at $2 ($(date -u +%FT%TZ)); closing."
}

# The run failed before the judge named a change: log $1, revision $2.
report_run_failure() {
  file_issue "$runner_title" "$runner_title" "\`cargo xtask scaling\` failed at $2 before the judge named a change ($(date -u +%FT%TZ)).

\`\`\`
$({ if grep -q 'FAIL perf:' "$1"; then sed -n '/FAIL perf:/,$p' "$1"; else tail -n 40 "$1"; fi; } | head -c 50000)
\`\`\`

Reproduce: \`cargo xtask scaling\` in a clean checkout at $2. Log on the dev machine: $1"
}

# Runs that ended without reporting ($state/UNREPORTED, written by
# --unit-failed), filed and then forgotten.
report_unreported() {
  [ -f "$state/UNREPORTED" ] || return 0
  file_issue "$runner_title" "$runner_title" "Nightly runs ended without reporting (the unit's OnFailure= record):

$(cat "$state/UNREPORTED")"
  rm -f "$state/UNREPORTED"
}
