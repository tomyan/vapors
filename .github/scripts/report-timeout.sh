#!/usr/bin/env bash
# The last step of every CI job, run only when the job is cancelled.
# GitHub shows a job that hit its timeout-minutes as "cancelled", the
# same as a cancel by hand or by a newer push; this says which it was
# and which step was running, then fails.
#
# Needs GH_TOKEN (the job's token; the workflow grants `actions: read`)
# and TIMEOUT_MINUTES (the job's timeout-minutes; `cargo xtask gate`
# refuses a job where they differ).
set -euo pipefail
: "${TIMEOUT_MINUTES:?set it to the timeout-minutes of this job}"
: "${GH_TOKEN:?set it to the token of this job}"

jobs_path="repos/$GITHUB_REPOSITORY/actions/runs/$GITHUB_RUN_ID/attempts/$GITHUB_RUN_ATTEMPT/jobs"
# A jobs read took under 1 s from the dev machine; 30 s leaves room.
if ! jobs=$(timeout 30 gh api --paginate "$jobs_path" --jq '.jobs[]' 2>&1); then
  echo "::error::this job was cancelled; reading its steps from $jobs_path failed: $jobs"
  exit 1
fi

# This job is the one still running on this runner. The report step
# is named "report a timeout" in every job; the gate checks that.
summary=$(jq -rs --arg runner "$RUNNER_NAME" --arg report "report a timeout" '
  map(select(.runner_name == $runner and .status != "completed")) | first
  | if . == null then "unknown" else
      ((now - (.started_at | fromdateiso8601)) | floor | tostring) + "\t" +
      ([.steps[] | select(.started_at != null and .name != $report)] | last | .name // "none")
    end' <<<"$jobs")
if [ "$summary" = unknown ]; then
  echo "::error::this job was cancelled; no running job on runner $RUNNER_NAME in $jobs_path"
  exit 1
fi

elapsed=${summary%%$'\t'*}
last_step=${summary#*$'\t'}
if [ "$elapsed" -ge $((TIMEOUT_MINUTES * 60)) ]; then
  echo "::error::this job hit its $TIMEOUT_MINUTES-minute timeout; the last step to start was \"$last_step\"; fix: find what was slow in that step's log, then re-measure the job's timeout-minutes"
else
  echo "::error::this job was cancelled after $elapsed s, before its $TIMEOUT_MINUTES-minute timeout (by hand, or by a newer run); the last step to start was \"$last_step\""
fi
exit 1
