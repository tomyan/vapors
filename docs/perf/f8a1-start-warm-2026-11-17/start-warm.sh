#!/bin/sh
# start-warm.sh <out dir>: App::start warm (Mesa shader cache filled by one discarded run per binary), main against the branch, 5 alternating runs each, load settled before each
set -eu
out=$1; mkdir -p "$out"
main=$HOME/.cache/w-lane/vapors-main/target/release/vapors-perf
branch=$HOME/.cache/w-lane/vapors/target/release/vapors-perf
cp "$main" "$out/perf-main"; cp "$branch" "$out/perf-branch"
. $HOME/.cache/w-lane/vapors/scripts/settle.sh
for b in main branch; do mkdir -p "$out/cache-$b"; MESA_SHADER_CACHE_DIR="$out/cache-$b" "$out/perf-$b" --first-frame 100 > "$out/fill-$b.txt" 2>&1; done
for i in 1 2 3 4 5; do for b in main branch; do
  settle
  MESA_SHADER_CACHE_DIR="$out/cache-$b" "$out/perf-$b" --first-frame 100 > "$out/$b-$i.txt" 2>&1
  echo "$b $i $(grep -o 'App::start to idle [0-9.]* µs' "$out/$b-$i.txt") frame0 $(grep -o 'frame 0 to idle [0-9.]* µs' "$out/$b-$i.txt") $(grep -o 'Shader cache: .*' "$out/$b-$i.txt" | grep -o '(warm)\|(cold)') load $(load)" | tee -a "$out/summary.txt"
done; done
rm -rf "$out"/cache-* "$out"/perf-*
