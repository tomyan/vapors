# Sourced by the page checks (check-page-contract.sh, web-wake-frames.sh,
# web-idle-frames.sh): which browser runs them, its own SwiftShader Vulkan
# driver, and the profile each launch runs in.
# browser: $CHROME, else google-chrome, else chromium (the runner image
# ships google-chrome).
browser() {
  for candidate in "${CHROME:-}" google-chrome chromium; do
    if [ -n "$candidate" ] && command -v "$candidate" >/dev/null 2>&1; then
      echo "$candidate"; return
    fi
  done
  echo "no browser: set CHROME, or install google-chrome or chromium" >&2
  exit 1
}

# The browser's own SwiftShader Vulkan driver, as the only driver, for
# the browser checks only; the gpu job's tests still run on lavapipe.
# With lavapipe as the only driver, the compositor cannot read the WebGPU
# canvas ("Unable to initialize SkSurface", "read from an uninitialized
# SharedImage") and the screenshot is white: run 39934974057, jobs
# 120352178274 and 120355716651, and here with VK_DRIVER_FILES set to
# lavapipe's lvp_icd.json. Nine flag variants under lavapipe alone failed:
# no Vulkan feature, --use-webgpu-adapter=swiftshader, --use-vulkan=
# swiftshader and =native, --disable-vulkan-surface, ANGLE on Vulkan with
# and without its features, Skia Graphite, and Vulkan off with the
# SwiftShader adapter. With this driver run 39937335978 passed three times.
# Validated paths, one per browser package (2026-10-23); a browser
# elsewhere is refused:
#   /opt/google/chrome/vk_swiftshader_icd.json  Google Chrome 155.0.8059.39 (the runner)
#   /usr/lib/chromium/vk_swiftshader_icd.json   Chromium 155.0.8059.79 (Debian 13)
# A browser update that moves or drops the file, or the library it names,
# fails here by name.
vulkan_driver() {
  binary=$(readlink -f "$(command -v "$1")")
  case $binary in
    /opt/google/chrome/*) icd=/opt/google/chrome/vk_swiftshader_icd.json ;;
    /usr/bin/chromium | /usr/lib/chromium/*) icd=/usr/lib/chromium/vk_swiftshader_icd.json ;;
    *) echo "$1 ($2) resolves to $binary, outside the browser packages this script has validated; fix: validate its vk_swiftshader_icd.json and add it to vulkan_driver in scripts/browser.sh" >&2; exit 1 ;;
  esac
  [ -r "$icd" ] || { echo "$1 ($2): no $icd, so no SwiftShader Vulkan driver; fix: find where this version ships vk_swiftshader_icd.json and update vulkan_driver in scripts/browser.sh" >&2; exit 1; }
  name=$(sed -n 's|.*"library_path": *"\./\([^"/]*\)".*|\1|p' "$icd")
  [ -n "$name" ] || { echo "$1 ($2): $icd has no \"library_path\": \"./<file>\" (its shape changed); fix: update vulkan_driver in scripts/browser.sh" >&2; exit 1; }
  library=$(dirname "$icd")/$name
  [ -f "$library" ] || { echo "$1 ($2): $icd names a library that is not at $library; fix: update vulkan_driver in scripts/browser.sh" >&2; exit 1; }
  echo "$icd"
}


# Days a browser profile is kept after the UTC day it is made: the
# automation cache's retention, the same as RETENTION_DAYS in
# crates/text/src/comparison/chromium_profile.rs (scripts/
# browser-profile-check.sh holds the two equal). The host's daily
# retention job removes a profile on the day after its date, once its scan
# finds no process using it.
PROFILE_RETENTION_DAYS=8

# expiry_profile: makes and prints a new directory
# ${XDG_CACHE_HOME:-$HOME/.cache}/chromium-headless/expires-YYYY-MM-DD-<id>,
# YYYY-MM-DD the UTC day PROFILE_RETENTION_DAYS from now. A launch given it
# as --user-data-dir makes no profile of its own under
# ~/.config/chromium-headless.
expiry_profile() {
  case ${XDG_CACHE_HOME:-} in
    /*) cache=$XDG_CACHE_HOME ;;
    *) case ${HOME:-} in
         /*) cache=$HOME/.cache ;;
         *) echo "no automation cache for a browser profile: neither XDG_CACHE_HOME nor HOME is an absolute path; fix: set HOME (or XDG_CACHE_HOME) to an absolute directory; reproduce: echo \"XDG_CACHE_HOME=\$XDG_CACHE_HOME HOME=\$HOME\"" >&2; return 1 ;;
       esac ;;
  esac
  root=$cache/chromium-headless
  expires=$(date -u -d "+$PROFILE_RETENTION_DAYS days" +%F) || { echo "date could not give the UTC day $PROFILE_RETENTION_DAYS days from now; fix: GNU date is needed; reproduce: date -u -d '+$PROFILE_RETENTION_DAYS days' +%F" >&2; return 1; }
  mkdir -p -- "$root" 2>/dev/null && mktemp -d "$root/expires-$expires-XXXXXXXX" 2>/dev/null || {
    echo "a browser profile under $root could not be made; fix: make that directory writable, or free space; reproduce: mkdir -p '$root' && mktemp -d '$root/expires-$expires-XXXXXXXX'" >&2
    return 1
  }
}

# Seconds in_profile waits, after the launch's command returns, for any
# process still carrying the profile's path (Chromium's children name it
# in --user-data-dir=) before killing it. Measured with Chromium 155 here
# (2026-11-22): none was left after 5 normal --dump-dom exits nor after 5
# exits on TERM (what `timeout` sends); after a KILL of the browser
# process its 9 children were gone within 103 to 173 ms (3 runs). 2 s is
# about 12 times the worst.
PROFILE_SETTLE_S=${PROFILE_SETTLE_S:-2}

# profile_processes: lists (pgrep -af) the processes naming $profile, its
# path matched literally; status 1 when there are none. A pgrep error
# stops the script, so a broken search is never read as "none left".
profile_processes() {
  profile_pattern=$(printf '%s' "$profile" | sed 's/[][\.*^$+?(){}|]/\\&/g')
  pgrep -af -- "$profile_pattern"
  profile_search=$?
  [ "$profile_search" -le 1 ] || { echo "pgrep failed ($profile_search) searching for processes of the browser profile $profile; fix: check pgrep (procps) works; reproduce: pgrep -af -- '$profile_pattern'" >&2; exit 1; }
  return "$profile_search"
}

# settle_profile: waits up to PROFILE_SETTLE_S for the processes naming
# $profile to end; any left are listed, killed, and the status is 1.
settle_profile() {
  settle_steps=$(awk -v s="$PROFILE_SETTLE_S" 'BEGIN { printf "%d", s * 20 }')
  while left=$(profile_processes); do
    if [ "$settle_steps" -le 0 ]; then
      echo "processes of the browser launch still running ${PROFILE_SETTLE_S} s after it, killed so its profile can be removed: $(printf '%s' "$left" | tr '\n' ';'); fix: find why the browser's processes outlived it; reproduce: run the check again" >&2
      printf '%s\n' "$left" | awk '{print $1}' | xargs -r kill -KILL 2>/dev/null || true
      sleep 0.2
      return 1
    fi
    settle_steps=$((settle_steps - 1))
    sleep 0.05
  done
}

# in_profile <command...>: runs the command with $profile naming a new
# expiry profile, then (settle_profile) waits for the launch's processes,
# removes the profile and returns the command's status (1 if it had to
# kill processes the launch left behind). A profile that
# cannot be removed is named and the shell it runs in exits 1: the script
# when called directly, only a $(...) subshell when called in one, whose
# caller then sees a non-zero status. A launch that is killed leaves its
# profile for the retention job.
in_profile() {
  profile=$(expiry_profile) || return 1
  in_profile_status=0
  "$@" || in_profile_status=$?
  settle_profile || [ "$in_profile_status" -ne 0 ] || in_profile_status=1
  if ! rm -rf -- "$profile" 2>/dev/null || [ -e "$profile" ]; then
    echo "the browser profile $profile could not be removed; fix: remove it by hand (the retention job removes it after its expiry date otherwise); reproduce: rm -r '$profile'" >&2
    exit 1
  fi
  return "$in_profile_status"
}
