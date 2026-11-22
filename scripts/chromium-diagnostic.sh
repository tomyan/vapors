#!/usr/bin/env bash
# DIAGNOSTIC (draft PR, not for main): the software stack under the
# text_line_box_paint 2x placement case, on one machine, for the pinned
# Chromium and a control browser. For each: the browser's build and
# hash, the exact launch (flags, scale), the libraries its processes
# load with their packages and hashes, the fonts the page used and what
# fontconfig resolves, the graphics backend on Chromium's path (WebGL's
# renderer string) and on Vapors' (the wgpu adapter), and the two sets
# of ink bounds, apart. Exits 0 whatever the case does: the case's own
# result is in each run.log; the script fails only when it cannot record.
#
#   scripts/chromium-diagnostic.sh <new dir>            # control: /usr/bin/chromium
#   CONTROL_CHROMIUM=/path/to/chrome scripts/chromium-diagnostic.sh <new dir>
set -euo pipefail
out=${1:?usage: scripts/chromium-diagnostic.sh <new dir>}
[ -e "$out" ] && { echo "chromium-diagnostic: $out exists; give a new directory" >&2; exit 1; }
mkdir -p "$out"
version=$(sed -n 's/^chromium.version = "\(.*\)"$/\1/p' .config/tools.toml)
pinned="$HOME/.local/share/factory/chromium-$version/chrome-linux/chrome"
[ -x "$pinned" ] || { echo "chromium-diagnostic: no pinned Chromium at $pinned; fix: cargo xtask setup" >&2; exit 1; }
control=${CONTROL_CHROMIUM:-/usr/bin/chromium}
# Vapors' side on the CI reference adapter on every machine (ci.yml's gpu
# job sets the same), so only Chromium's side varies.
export VAPORS_GPU_ADAPTER=${VAPORS_GPU_ADAPTER:-llvmpipe}
target=$(cargo metadata --format-version 1 --no-deps | python3 -I -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')
work="$target/tmp/paint-placement-2x"

package_of() {
  local owner
  owner=$(dpkg -S "$1" 2>/dev/null | head -1 | cut -d: -f1) || true
  if [ -n "$owner" ]; then echo "$owner $(dpkg-query -W -f='${Version}' "$owner")"; else echo "no package"; fi
}

machine() {
  echo "== machine"
  sed -n 's/^PRETTY_NAME=//p' /etc/os-release
  uname -srm
  grep -m1 'model name' /proc/cpuinfo
  echo "cpu flags: $(grep -m1 '^flags' /proc/cpuinfo | tr ' ' '\n' | grep -E '^(sse4_2|avx|avx2|avx512f|avx512bw|fma|f16c)$' | tr '\n' ' ')"
  echo "nproc $(nproc)"
  echo "== packages"
  for p in libfontconfig1 fontconfig-config libfreetype6 libharfbuzz0b libvulkan1 mesa-vulkan-drivers libegl1 libgbm1 libnss3 libx11-6 libexpat1 libglib2.0-0t64 libglib2.0-0; do
    v=$(dpkg-query -W -f='${Version}' "$p" 2>/dev/null) && echo "$p $v"
  done
  echo "== vulkan (Vapors' path)"
  vulkaninfo --summary 2>/dev/null | grep -E 'deviceName|driverName|driverInfo|apiVersion' || echo "vulkaninfo unavailable"
  echo "VK_ICD_FILENAMES=${VK_ICD_FILENAMES:-} VAPORS_GPU_ADAPTER=${VAPORS_GPU_ADAPTER:-}"
  echo "== fontconfig"
  fc-match --version 2>&1 | head -1
  for f in Geist "Geist Mono" sans-serif monospace; do echo "fc-match '$f': $(fc-match -f '%{file} %{family} %{hintstyle} %{antialias} %{rgba} %{lcdfilter}\n' "$f")"; done
}

# Every shared library mapped by processes whose executable is $1, until $2 exists.
sample_libraries() {
  local exe=$1 stop=$2
  while [ ! -e "$stop" ]; do
    for p in $(pgrep -f -- "^$exe" || true); do
      # a process may end between pgrep and the read: nothing to record
      awk '$6 ~ /\.so/ {print $6}' "/proc/$p/maps" 2>/dev/null || true
    done
    sleep 0.02
  done | sort -u
}

case_run() {
  local which=$1 browser=$2 dir="$out/$1"
  mkdir -p "$dir"
  local exe; exe=$(readlink -f "$browser")
  # Debian's /usr/bin/chromium is a wrapper script for /usr/lib/chromium/chromium.
  if head -c2 "$exe" | grep -q '#!'; then exe=/usr/lib/chromium/chromium; fi
  {
    echo "== browser ($which)"
    echo "launched $browser"
    echo "executable $exe sha256 $(sha256sum "$exe" | cut -d' ' -f1)"
    echo "version $("$browser" --version 2>&1 | head -1)"
  } > "$dir/browser.txt"
  sample_libraries "$exe" "$dir/stop" > "$dir/libraries.raw" &
  local sampler=$!
  set +e
  VAPORS_CHROMIUM="$browser" VAPORS_CHROMIUM_TRACE=1 PLACEMENT_OUT="$dir" VAPORS_TEST_TIER=gpu \
    cargo nextest run -p vapors-elements-corpus --test gpu_paint --no-capture \
    -E 'test(gpu_at_2x_placement::text_line_box_paint)' > "$dir/run.log" 2>&1
  echo "case exit $?" >> "$dir/browser.txt"
  set -e
  touch "$dir/stop"; wait "$sampler"
  while read -r lib; do
    echo "$lib $(sha256sum "$lib" 2>/dev/null | cut -c1-16) $(case $lib in "$(dirname "$exe")"/*) echo bundled;; *) package_of "$lib";; esac)"
  done < "$dir/libraries.raw" > "$dir/libraries.txt"
  grep -E '^(CHROMIUM|PLACEMENT)|^\s+(PASS|FAIL) ' "$dir/run.log" >> "$dir/browser.txt" || true
  {
    echo "== fonts the page loaded ($work)"
    for f in "$work"/*.ttf; do echo "$(basename "$f") $(sha256sum "$f" | cut -d' ' -f1)"; done
  } >> "$dir/browser.txt"
  local profile; profile=$(mktemp -d)
  timeout 60 "$browser" --headless=new --no-sandbox --hide-scrollbars --force-device-scale-factor=2 \
    --virtual-time-budget=5000 --window-size=640,400 --user-data-dir="$profile" --dump-dom \
    "file://$PWD/scripts/chromium-diagnostic.html" 2>/dev/null \
    | python3 -I -c 'import re,sys; m=re.search(r"<body>(.*?)<", sys.stdin.read(), re.S); print("graphics", m.group(1).strip() if m else "not reported")' >> "$dir/browser.txt"
  rm -rf "$profile"
}

machine > "$out/machine.txt" 2>&1
case_run pinned "$pinned"
case_run control "$control"
echo "chromium-diagnostic: written to $out"
