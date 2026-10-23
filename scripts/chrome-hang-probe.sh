#!/bin/bash
# Diagnostic only (not for merge): launch headless Chrome on the text
# comparison page many times, as vapors-text's launcher does, and record
# how long each takes. Variants: the launcher's flags; without
# --virtual-time-budget; the launcher's flags with verbose logging.
# Usage: scripts/chrome-hang-probe.sh <page.html> <out dir> <runs>
set -u
page=$1; out=$2; runs=$3
mkdir -p "$out"
chrome=$(command -v google-chrome || command -v chromium)
echo "browser: $chrome ($($chrome --version))"
echo "DBUS_SESSION_BUS_ADDRESS=${DBUS_SESSION_BUS_ADDRESS-<unset>} DBUS_SYSTEM_BUS_ADDRESS=${DBUS_SYSTEM_BUS_ADDRESS-<unset>} DISPLAY=${DISPLAY-<unset>} XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR-<unset>}"
base=(--headless=new --no-sandbox --hide-scrollbars --force-device-scale-factor=1 --window-size=1280,580)
probe() {  # probe <variant> <extra flags...>
  local variant=$1; shift
  local slow=0 failed=0 times=()
  for i in $(seq 1 "$runs"); do
    local png="$out/$variant-$i.png" log="$out/$variant-$i.log"
    rm -f "$png"
    local start=$(date +%s.%N)
    env -u DBUS_SESSION_BUS_ADDRESS ${PROBE_ENV:-} timeout -s KILL 30 "$chrome" "${base[@]}" "$@" --screenshot="$png" "file://$page" > /dev/null 2> "$log"
    local code=$?
    local secs=$(echo "$(date +%s.%N) - $start" | bc)
    times+=("$(printf '%.1f' "$secs")")
    if [ "$code" != 0 ] || [ ! -s "$png" ]; then failed=$((failed + 1)); echo "  $variant run $i: exit $code after ${secs}s, png $( [ -s "$png" ] && echo yes || echo no)"; fi
    if (( $(echo "$secs > 5" | bc) )); then slow=$((slow + 1)); else rm -f "$log"; fi
    rm -f "$png"
  done
  echo "$variant: $runs runs, $failed failed, $slow over 5 s; seconds: ${times[*]}"
}
probe unset --virtual-time-budget=5000
PROBE_ENV=DBUS_SESSION_BUS_ADDRESS=disabled: probe dbus-disabled --virtual-time-budget=5000
probe scale-2x --virtual-time-budget=5000 --force-device-scale-factor=2
# Under load: one busy loop per core, as the gpu tier's other tests
# (lavapipe) load the runner while the comparison test runs Chrome.
loads=()
for _ in $(seq 1 "$(nproc)"); do (while :; do :; done) & loads+=($!); done
probe loaded-unset --virtual-time-budget=5000 --enable-logging=stderr --v=0
PROBE_ENV=DBUS_SESSION_BUS_ADDRESS=disabled: probe loaded-dbus-disabled --virtual-time-budget=5000
probe loaded-no-virtual-time
kill "${loads[@]}"
ls "$out" | head -50
