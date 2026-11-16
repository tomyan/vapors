#!/bin/sh
# Property 7's record (docs/plans/build-time-styles.md, M1): the Mount
# page's first frame as a user waits for it, `vapors-perf --first-frame
# <rows>` (mount, App::start to the GPU's idle, frame 0 to idle), one
# page a process, at 1,000, 3,000 and 10,000 rows:
# - times on release, the shipped profile: three pairs a size, each a
#   cold run on a new, empty Mesa shader cache, then a warm run on the
#   cache it filled (as scene runs are, scripts/scene-runs.sh);
# - instructions on the judged profile (codegen-units 1), which compare
#   across revisions: three runs a size, on one cache the first fills.
# Each run's output is kept verbatim with a log of its start, end, exit
# code and load, and the binaries' sha256; each run waits for the load
# to settle first (scripts/settle.sh). scripts/first-frame-table.sh
# makes the tables at the end, so a run it refuses fails this script.
#
# Usage: scripts/first-frame-runs.sh <new or empty dir>
# On a clean tree only. Run it under
# `cargo xtask quiet 30 -- scripts/first-frame-runs.sh <dir>`: the two
# builds took about 3 min each with a warm sccache (2026-11-16), settle
# waits at most 900 s before a run, and one run took 1.7 s at 10,000
# rows (cold cache, load 7.8, 2026-11-16), so the 27 runs take about a
# minute once the load is down; 30 min holds the builds, one full
# settle and the runs.
set -eu
out=${1:?usage: scripts/first-frame-runs.sh <new or empty dir>}
here=$(dirname "$0")
# a run's deadline: 35 times the 1.7 s measured above
deadline=60
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE
if [ -n "$(git status --porcelain)" ]; then
  echo "the tree is dirty; commit, then: scripts/first-frame-runs.sh $out" >&2; exit 1
fi
if [ -e "$out" ] && [ -n "$(ls -A "$out")" ]; then
  echo "$out is not empty; give a new directory: scripts/first-frame-runs.sh <new dir>" >&2; exit 1
fi
command -v cargo > /dev/null || { echo "cargo is not on PATH; fix: add ~/.cargo/bin to PATH" >&2; exit 1; }
revision=$(git describe --always --dirty --abbrev=12)
mkdir -p "$out"
# the target directory cargo builds into, wherever CARGO_TARGET_DIR or a
# config puts it, so the binaries copied are the ones just built
target=$(cargo metadata --format-version 1 --no-deps | python3 -I -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')
cargo build --release -p vapors-perf
cargo build --profile judged -p vapors-perf
cp "$target/release/vapors-perf" "$out/release-vapors-perf"
cp "$target/judged/vapors-perf" "$out/judged-vapors-perf"
(cd "$out" && sha256sum release-vapors-perf judged-vapors-perf > sha256.txt)
. "$here/settle.sh"
run() { # run <name> <profile> <rows> <cache dir>
  settle
  echo "$1 start $(date +%T) load $(load)" >> "$out/log.txt"
  code=0
  MESA_SHADER_CACHE_DIR=$4 timeout "$deadline" "$out/$2-vapors-perf" --first-frame "$3" > "$out/$1.txt" 2>&1 || code=$?
  echo "$1 end $(date +%T) exit $code load $(load)" >> "$out/log.txt"
  if [ "$code" != 0 ]; then
    echo "$out/$1.txt: the run failed (exit $code; 124 is the ${deadline} s deadline); reproduce: MESA_SHADER_CACHE_DIR=$4 $out/$2-vapors-perf --first-frame $3" >&2
    exit 1
  fi
  ran=$(sed -n 's/^Revision: //p' "$out/$1.txt")
  if [ "$ran" != "$revision" ]; then
    echo "$out/$1.txt: the binary is of $ran, but HEAD is $revision; fix: check that cargo built into $target" >&2
    exit 1
  fi
}
for rows in 1000 3000 10000; do
  for pair in 1 2 3; do
    cache="$out/cache-$rows-$pair"; mkdir "$cache"
    run "release-$rows-$pair-cold" release "$rows" "$cache"
    run "release-$rows-$pair-warm" release "$rows" "$cache"
    rm -rf "$cache"
  done
done
# the phases' instructions do not include pipeline builds, which
# App::start makes outside them; one cache keeps every judged run alike
mkdir "$out/cache-judged"
for rows in 1000 3000 10000; do
  for n in 1 2 3; do
    run "judged-$rows-$n" judged "$rows" "$out/cache-judged"
  done
done
rm -rf "$out/cache-judged" "$out/release-vapors-perf" "$out/judged-vapors-perf"
cat "$out/log.txt"
"$here/first-frame-table.sh" "$revision" "$out"/release-*.txt "$out"/judged-*.txt > "$out/tables.md"
cat "$out/tables.md"
