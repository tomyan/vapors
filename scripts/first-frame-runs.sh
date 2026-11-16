#!/bin/sh
# Property 7's record (docs/plans/build-time-styles.md, M1): three runs
# of `vapors-perf 9 --mount`, the first frame of a compiled `{for}` at
# 1,000, 3,000 and 10,000 rows, on release, the shipped profile. Each
# run's stdout and stderr is kept verbatim in the directory, with a log
# of each run's start, end, exit code and /proc/loadavg, and the
# binary's sha256; scripts/first-frame-table.sh makes the tables from
# them and is run at the end, so a run it refuses (a load of 1.5 or
# more at a start or end, a software adapter, another profile) fails
# this script, naming the file.
#
# Usage: scripts/first-frame-runs.sh <new or empty dir>
# On a clean tree only: the harness prints the revision it was built
# from, and a dirty one could not be traced. Run it under
# `cargo xtask quiet 20 -- scripts/first-frame-runs.sh <dir>`, which
# holds xtask's ci, test and gate runs; before the first run it waits
# for the 1-minute load to fall under 1.5 (scripts/settle.sh). One run
# took 36 s at 3 pages a size (build-time-styles.md, section 5).
set -eu
out=${1:?usage: scripts/first-frame-runs.sh <new or empty dir>}
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE
if [ -n "$(git status --porcelain)" ]; then
  echo "the tree is dirty; commit, then: scripts/first-frame-runs.sh $out" >&2; exit 1
fi
if [ -e "$out" ] && [ -n "$(ls -A "$out")" ]; then echo "$out: not empty" >&2; exit 1; fi
mkdir -p "$out"
cargo build --release -p vapors-perf
cp target/release/vapors-perf "$out/vapors-perf"
(cd "$out" && sha256sum vapors-perf > sha256.txt)
. "$(dirname "$0")/settle.sh"
settle
for name in run1 run2 run3; do
  echo "$name start $(date +%T) load $(load)" >> "$out/log.txt"
  code=0
  "$out/vapors-perf" 9 --mount > "$out/$name.txt" 2>&1 || code=$?
  echo "$name end $(date +%T) exit $code load $(load)" >> "$out/log.txt"
  [ "$code" = 0 ] || { echo "$out/$name.txt: the run failed (exit $code); see that file" >&2; exit 1; }
done
rm "$out/vapors-perf"
cat "$out/log.txt"
"$(dirname "$0")/first-frame-table.sh" "$out/run1.txt" "$out/run2.txt" "$out/run3.txt" > "$out/tables.md"
cat "$out/tables.md"
