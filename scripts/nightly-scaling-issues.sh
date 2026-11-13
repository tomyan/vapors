# Sourced by scripts/nightly-scaling.sh: the GitHub issues it files, from
# the lines vapors-perf prints (scaling_all.rs; pinned by
# scripts/fixtures/nightly-scaling.log). Issues are found by title prefix
# in a plain listing, not the search index, which lags a create.
# - "Nightly scaling: <change>: <kind> <phase>, ...": one per failing
#   change; retitled when its parts change, closed when it passes or when
#   the harness no longer lists the change.
# - $runner_title: a run that did not judge every change (the runner, the
#   build, a deadline, a crash); closed when every change passes.
# - $unreported_title: runs that ended without reporting; closed by hand.

runner_title="Nightly scaling judge failing on main"
unreported_title="Nightly scaling: run ended without reporting"
change_prefix="Nightly scaling: "

github() { timeout "$network_deadline" "$gh" "$@"; }

# "<number>\t<title>" of each open issue whose title starts with $1.
open_issues() {
  github issue list -R "$repo" --state open --limit 200 --json number,title \
    --jq ".[] | select(.title | startswith(\"$1\")) | \"\(.number)\t\(.title)\""
}

# Comments on the open issue whose title starts with $1, retitled $2 when
# it differs, or opens one titled $2; body $3.
file_issue() {
  local found number old
  found=$(open_issues "$1" | head -n 1)
  if [ -z "$found" ]; then
    github issue create -R "$repo" --title "$2" --body "$3"
    return
  fi
  number=${found%%$'\t'*} old=${found#*$'\t'}
  if [ "$old" = "$2" ]; then
    github issue comment "$number" -R "$repo" --body "$3"
    return
  fi
  github issue edit "$number" -R "$repo" --title "$2"
  github issue comment "$number" -R "$repo" --body "Retitled; was \"$old\".

$3"
}

# Closes the open issue whose title starts with $1, if any, with comment $2.
close_issue() {
  local found
  found=$(open_issues "$1" | head -n 1)
  [ -z "$found" ] && return 0
  github issue close "${found%%$'\t'*}" -R "$repo" --comment "$2"
}

# The changes whose `scaling-result:` line in log $1 says $2 (a regex).
changes_that() { { grep -oE "scaling-result: [A-Za-z]+ $2" "$1" || true; } | cut -d' ' -f2 | sort -u; }

# The changes the `scaling-changes:` line of log $1 names, sorted.
expected_changes() {
  { grep -m 1 -E '^ *scaling-changes:' "$1" || true; } | cut -d: -f2 | xargs -n 1 | sort -u
}

# Change $2's error in log $1, between its begin and end lines.
change_block() {
  sed 's/^ *//' "$1" | sed -n "/^scaling-error-begin: $2\$/,/^scaling-error-end: $2\$/p" |
    sed '1d;$d' | head -c 50000
}

# "<kind> <phase>, ..." of a growth failure on stdin, cut to fit a title:
# GitHub refuses one over 256 characters.
failing_parts() {
  awk '/^scaling: / { kind = $0; sub(/^scaling: [A-Za-z]+,? ?/, "", kind); sub(/ ?does work.*/, "", kind) }
       /^- / { phase = $0; sub(/^- /, "", phase); sub(/ (grows|costs) .*/, "", phase)
               print (kind == "" ? "" : kind " ") phase }' | sort -u |
    awk '{ if (length(out) + length($0) < 180) out = out (NR > 1 ? ", " : "") $0; else more++ }
         END { printf "%s", out; if (more) printf ", and %d more", more }'
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
  local block parts
  block=$(change_block "$2" "$1")
  [ -n "$block" ] || block="No scaling-error block for $1 in the log; its last lines:
$(tail -n 40 "$2")"
  parts=$(failing_parts <<<"$block")
  file_issue "$change_prefix$1:" "$change_prefix$1: ${parts:-failed, not by growth (see the body)}" \
    "\`--scaling $1\` failed at $3 ($(date -u +%FT%TZ)).

$(since_last_pass "$1" "$3")

\`\`\`
$block
\`\`\`

Reproduce: \`cargo build --release -p vapors-perf && target/release/vapors-perf 60 10 --scaling $1\` in a clean checkout at $3. Log on the dev machine: $2"
}

# Change $1 passed at revision $2.
report_change_pass() {
  echo "$2" >"$state/pass/$1"
  close_issue "$change_prefix$1:" "\`--scaling $1\` passed at $2 ($(date -u +%FT%TZ)); closing."
}

# Closes the open change issues of changes the harness no longer lists
# ($1) at revision $2.
report_retired() {
  local number title change
  while IFS=$'\t' read -r number title; do
    change=$(sed -nE "s/^${change_prefix}([A-Za-z]+): .*/\1/p" <<<"$title")
    if [ -z "$change" ] || grep -qx "$change" <<<"$1"; then continue; fi
    github issue close "$number" -R "$repo" \
      --comment "vapors-perf no longer judges $change (renamed or removed?) at $2; closing."
  done < <(open_issues "$change_prefix")
}

# The run at revision $2 did not judge every change: log $1; $3 names the
# changes with no result.
report_unjudged() {
  file_issue "$runner_title" "$runner_title" "The run at $2 did not judge every change ($(date -u +%FT%TZ)): ${3:-no scaling-changes line, so the build failed, the judge did not start, or vapors-perf's lines changed}.

\`\`\`
$({ if grep -q 'FAIL perf:' "$1"; then sed -n '/FAIL perf:/,$p' "$1"; else tail -n 40 "$1"; fi; } | head -c 50000)
\`\`\`

Reproduce: \`cargo xtask scaling\` in a clean checkout at $2. Log on the dev machine: $1"
}

# Runs that ended without reporting ($state/UNREPORTED), on their own
# issue, which no pass closes; then forgotten.
report_unreported() {
  [ -f "$state/UNREPORTED" ] || return 0
  file_issue "$unreported_title" "$unreported_title" "Nightly runs ended without reporting:

$(cat "$state/UNREPORTED")"
  rm -f "$state/UNREPORTED"
}

# Every report for log $1 at revision $2, the judge having exited $3.
report_results() {
  local change expected failing missing
  report_unreported
  expected=$(expected_changes "$1")
  for change in $(changes_that "$1" pass); do report_change_pass "$change" "$2"; done
  failing=$(changes_that "$1" fail)
  for change in $failing; do report_change_failure "$change" "$1" "$2"; done
  missing=$(comm -23 <(echo "$expected") <(changes_that "$1" '(pass|fail)') | xargs)
  if [ -z "$expected" ] || [ -n "$missing" ] || { [ "$3" -ne 0 ] && [ -z "$failing" ]; }; then
    report_unjudged "$1" "$2" "${missing:+no result for $missing}"
    return 0
  fi
  report_retired "$expected" "$2"
  [ "$3" -ne 0 ] || close_issue "$runner_title" "\`cargo xtask scaling\` passed at $2; closing."
}
