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

# in_profile <command...>: runs the command with $profile naming a new
# expiry profile, removes the profile after, and returns the command's
# status. A profile that cannot be removed is named and the script (or
# the subshell it runs in) exits 1. A launch that is killed leaves its
# profile for the retention job.
in_profile() {
  profile=$(expiry_profile) || return 1
  in_profile_status=0
  "$@" || in_profile_status=$?
  if ! rm -rf -- "$profile" 2>/dev/null || [ -e "$profile" ]; then
    echo "the browser profile $profile could not be removed; fix: remove it by hand (the retention job removes it after its expiry date otherwise); reproduce: rm -r '$profile'" >&2
    exit 1
  fi
  return "$in_profile_status"
}
