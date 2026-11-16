#!/bin/sh
# A scene's 60 s run as the finish line records it (one-scene 3.3):
# three pairs of `vapors-perf --scene-run <scene>` (racer by default, or
# one-scene), each a cold run on a new,
# empty Mesa shader cache and then a warm run on the cache the cold run
# filled. Each run's stdout and stderr is kept verbatim in the
# directory, with a log of each run's start, end, exit code and
# /proc/loadavg, and the binary's sha256; scripts/scene-run-table.sh
# makes the tables from them.
#
# Usage: scripts/scene-runs.sh <new or empty dir> [racer|one-scene]
# On a clean tree only: the harness prints the revision it was built
# from, and a dirty one could not be traced. Needs a quiet machine
# (each run refuses a 1-minute load of 1.5 or more): run it under
# `cargo xtask quiet <minutes> -- scripts/scene-runs.sh <dir> <scene>`,
# which holds xtask's ci, test and gate runs. Before the first run it
# waits for the 1-minute load to fall under 1.5 (scripts/settle.sh).
# A run exits 1 when a frame is over budget; that is recorded, not a failure of the script.
# A run that never reached its first frame (a refusal: the load, the
# adapter, the shader cache) fails the script, naming its output.
set -eu
out=${1:?usage: scripts/scene-runs.sh <new or empty dir> [racer|one-scene]}
scene=${2:-racer}
# The scenes are vapors-perf's SceneKind (crates/perf/src/scene_kind.rs);
# a scene added there is added here too.
case $scene in racer|one-scene) ;; *) echo "no scene is called $scene; give racer or one-scene" >&2; exit 2 ;; esac
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE
if [ -n "$(git status --porcelain)" ]; then
  echo "the tree is dirty; commit, then: scripts/scene-runs.sh $out $scene" >&2; exit 1
fi
if [ -e "$out" ] && [ -n "$(ls -A "$out")" ]; then echo "$out: not empty" >&2; exit 1; fi
mkdir -p "$out"
cargo build --release -p vapors-perf
cp target/release/vapors-perf "$out/vapors-perf"
(cd "$out" && sha256sum vapors-perf > sha256.txt)
. "$(dirname "$0")/settle.sh"
run() { # run <name> <cache dir>
  echo "$1 start $(date +%T) load $(load)" >> "$out/log.txt"
  code=0
  MESA_SHADER_CACHE_DIR=$2 "$out/vapors-perf" --scene-run "$scene" > "$out/$1.txt" 2>&1 || code=$?
  echo "$1 end $(date +%T) exit $code load $(load)" >> "$out/log.txt"
}
settle
for pair in 1 2 3; do
  cache="$out/cache-$pair"; mkdir "$cache"
  run "run$pair-cold" "$cache"
  run "run$pair-warm" "$cache"
  rm -rf "$cache"
done
rm "$out/vapors-perf"
cat "$out/log.txt"
# A run prints its `Frames:` line only once past its refusals.
for name in run1-cold run1-warm run2-cold run2-warm run3-cold run3-warm; do
  grep -q '^Frames: ' "$out/$name.txt" \
    || { echo "$out/$name.txt: the run was refused before frame 0; see that file" >&2; exit 1; }
done
