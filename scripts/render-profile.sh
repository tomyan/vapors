#!/bin/sh
# The render profile of queue item 13b (docs/perf/render-profile-13b.md):
# `vapors-perf --scene-run <scene> --profile-only` for the racer and the
# one-scene example, at 1 and 4 samples, at the scene's own size and at
# 1920x1080, on ANV (the UHD 630) and on llvmpipe; then the same scenes
# on ANV again under Mesa's INTEL_MEASURE=draw, each draw's GPU time
# summarised per pass and pipeline by scripts/intel_measure_table.py.
# The INTEL_MEASURE runs are separate: measuring each draw slows them.
#
# Usage: scripts/render-profile.sh <new or empty dir> anv|llvmpipe|draws
# One part a call (a quiet hold lasts at most 60 minutes): the runs on
# ANV, the runs on llvmpipe, or the INTEL_MEASURE runs.
# On a clean tree only (the harness prints its revision). Needs a quiet
# machine: run it under `cargo xtask quiet <minutes> -- scripts/render-profile.sh <dir>`;
# each run waits for a 1-minute load under 1.5 (scripts/settle.sh) and
# refuses one at or over it at its start and end. Each adapter has its
# own Mesa shader cache, filled by its part's first run: that run's
# frame 0 is cold, every later one warm. A run that never reached its first frame
# fails the script, naming its output.
set -eu
usage="usage: scripts/render-profile.sh <new or empty dir> anv|llvmpipe|draws"
out=${1:?$usage}
part=${2:?$usage}
case $part in anv|llvmpipe|draws) ;; *) echo "no part is called $part; $usage" >&2; exit 2 ;; esac
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE
if [ -n "$(git status --porcelain)" ]; then
  echo "the tree is dirty; commit, then: scripts/render-profile.sh $out $part" >&2; exit 1
fi
if [ -e "$out" ] && [ -n "$(ls -A "$out")" ]; then echo "$out: not empty" >&2; exit 1; fi
cache="$out/cache"
mkdir -p "$cache"
cargo build --release -p vapors-perf
cp target/release/vapors-perf "$out/vapors-perf"
(cd "$out" && sha256sum vapors-perf > sha256.txt)
. "$(dirname "$0")/settle.sh"
run() { # run <name> <adapter: anv|llvmpipe> <scene> <samples> <size or own> [INTEL_MEASURE csv]
  name=$1 adapter=$2 scene=$3 samples=$4 size=$5 measure=${6:-}
  set -- --scene-run "$scene" --samples "$samples" --profile-only
  [ "$size" = own ] || set -- "$@" --size "$size"
  pick=""; [ "$adapter" = llvmpipe ] && pick=llvmpipe
  settle
  echo "$name start $(date +%T) load $(load)" >> "$out/log.txt"
  code=0
  env ${measure:+INTEL_MEASURE=draw,file=$measure,count=100000} VAPORS_GPU_ADAPTER="$pick" \
    MESA_SHADER_CACHE_DIR="$cache" "$out/vapors-perf" "$@" > "$out/$name.txt" 2>&1 || code=$?
  echo "$name end $(date +%T) exit $code load $(load)" >> "$out/log.txt"
  grep -q '^Frames: ' "$out/$name.txt" \
    || { echo "$out/$name.txt: the run was refused before frame 0; see that file" >&2; exit 1; }
}
matrix() { # matrix <adapter>
  for scene in racer one-scene; do
    for samples in 1 4; do
      for size in own 1920x1080; do
        run "$1-$scene-${samples}x-$size" "$1" "$scene" "$samples" "$size"
      done
    done
  done
}
draws() {
for scene in racer one-scene; do
  for samples in 1 4; do
    for size in own 1920x1080; do
      name="draws-$scene-${samples}x-$size"
      run "$name" anv "$scene" "$samples" "$size" "$out/$name.csv"
      python3 -I "$(dirname "$0")/intel_measure_table.py" "$out/$name.csv" > "$out/$name.md"
    done
  done
done
}
case $part in draws) draws ;; *) matrix "$part" ;; esac
rm -rf "$out/vapors-perf" "$cache"
cat "$out/log.txt"
