#!/bin/bash
# Diagnostic only (not for merge): the first Chrome launch on a fresh
# runner. With --pre-read, Chrome's install is read into the page cache
# first (timed), to test whether the cold launch is disk reads.
# With --fc-list, fontconfig's cache is built first (timed). Each launch
# renders text in a system font, as the comparison page does.
# Usage: scripts/chrome-cold-probe.sh <out dir> [--pre-read | --fc-list]
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
if [ "${2:-}" = --fc-list ]; then
  start=$(date +%s.%N)
  fc-list > /dev/null
  echo "fc-list (fontconfig's cache): $(echo "$(date +%s.%N) - $start" | bc) s"
fi
ls -d ~/.cache/fontconfig 2>/dev/null && echo "user fontconfig cache: $(ls ~/.cache/fontconfig | wc -l) files" || echo "no user fontconfig cache"
page="data:text/html,<p style='font-family:sans-serif'>Hello, cold start</p>"
for i in 1 2 3; do
  start=$(date +%s.%N)
  env -u DBUS_SESSION_BUS_ADDRESS timeout -s KILL 300 "$chrome" --headless=new --no-sandbox --hide-scrollbars \
    --window-size=640,400 --enable-logging=stderr --v=1 --screenshot="$out/text-$i.png" "$page" > /dev/null 2> "$out/cold-$i.log"
  echo "launch $i: exit $? after $(echo "$(date +%s.%N) - $start" | bc) s"
done
