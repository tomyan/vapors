#!/bin/bash
# Diagnostic only (not for merge): the first Chrome launch on a fresh
# runner. With --pre-read, Chrome's install is read into the page cache
# first (timed), to test whether the cold launch is disk reads.
# Usage: scripts/chrome-cold-probe.sh <out dir> [--pre-read]
set -u
out=$1; mkdir -p "$out"
chrome=$(command -v google-chrome || command -v chromium)
dir=$(dirname "$(readlink -f "$chrome")")
echo "browser: $chrome -> $dir, $(du -sh "$dir" | cut -f1)"
if [ "${2:-}" = --pre-read ]; then
  start=$(date +%s.%N)
  find "$dir" -type f -exec cat {} + > /dev/null
  echo "pre-read of $dir: $(echo "$(date +%s.%N) - $start" | bc) s"
fi
for i in 1 2 3; do
  start=$(date +%s.%N)
  env -u DBUS_SESSION_BUS_ADDRESS timeout -s KILL 300 "$chrome" --headless=new --no-sandbox --hide-scrollbars \
    --window-size=640,400 --enable-logging=stderr --v=1 --screenshot="$out/blank-$i.png" about:blank > /dev/null 2> "$out/cold-$i.log"
  echo "launch $i: exit $? after $(echo "$(date +%s.%N) - $start" | bc) s"
done
