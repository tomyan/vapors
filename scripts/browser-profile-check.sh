#!/usr/bin/env bash
# Proves scripts/browser.sh's expiry-named browser profiles, and the page
# checks' use of them, with no browser: where a profile is made and what it
# is named, the refusals, its removal after the launch however it ends,
# the retention held equal to the Rust launcher's, and that every headless
# launch in the page checks runs in one. Exits non-zero naming each guard
# that failed.
#
# Usage: scripts/browser-profile-check.sh
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
work=$(mktemp -d)
trap 'chmod -R u+w "$work" 2>/dev/null; rm -rf "$work"' EXIT
failed=0
fail() { echo "FAIL $*" >&2; failed=1; }
. "$here/browser.sh"

utc_in_8_days() {
  python3 -c 'import datetime as d; print((d.datetime.now(d.timezone.utc) + d.timedelta(days=8)).date())'
}

# A profile under $XDG_CACHE_HOME/chromium-headless, named for the UTC day
# 8 days on (either side of a midnight crossed during the call).
before=$(utc_in_8_days)
made=$(XDG_CACHE_HOME="$work/cache" expiry_profile) || fail "expiry_profile refused an absolute XDG_CACHE_HOME"
after=$(utc_in_8_days)
name=$(basename "${made:-none}")
[ "$(dirname "${made:-none}")" = "$work/cache/chromium-headless" ] || fail "profile $made is not under $work/cache/chromium-headless"
[[ $name =~ ^expires-($before|$after)-[A-Za-z0-9]{8}$ ]] || fail "profile name $name is not expires-$before-<8 letters or digits>"
[ -d "${made:-none}" ] || fail "profile $made was not made"

# XDG_CACHE_HOME unset or relative: under $HOME/.cache, as XDG says.
made=$(XDG_CACHE_HOME=relative HOME="$work/home" expiry_profile) || fail "expiry_profile refused a relative XDG_CACHE_HOME with an absolute HOME"
[ "$(dirname "${made:-none}")" = "$work/home/.cache/chromium-headless" ] || fail "with a relative XDG_CACHE_HOME the profile $made is not under \$HOME/.cache/chromium-headless"

# Neither absolute: refused, naming both.
if XDG_CACHE_HOME= HOME=relative expiry_profile >/dev/null 2>"$work/err"; then
  fail "expiry_profile made a profile with neither XDG_CACHE_HOME nor HOME absolute"
fi
grep -q XDG_CACHE_HOME "$work/err" && grep -q HOME "$work/err" || fail "the refusal does not name XDG_CACHE_HOME and HOME: $(cat "$work/err")"

# in_profile: the command sees $profile made, it is gone after, and the
# command's status is returned.
look() { [ -d "$profile" ] && echo "$profile" >"$work/seen"; return "$1"; }
XDG_CACHE_HOME="$work/cache" in_profile look 0 || fail "in_profile returned non-zero for a command that exited 0"
seen=$(cat "$work/seen" 2>/dev/null || echo none)
[ "$seen" != none ] || fail "the command did not see its profile made"
[ ! -e "$seen" ] || fail "in_profile left $seen after a command that exited 0"
rm -f "$work/seen"
status=0
XDG_CACHE_HOME="$work/cache" in_profile look 3 || status=$?
[ "$status" = 3 ] || fail "in_profile returned $status for a command that exited 3"
seen=$(cat "$work/seen" 2>/dev/null || echo none)
[ ! -e "$seen" ] || fail "in_profile left $seen after a command that exited 3"

# A profile that cannot be removed: named, and the script exits 1.
lock() { mkdir "$profile/locked" && touch "$profile/locked/f" && chmod 500 "$profile/locked"; }
status=0
(XDG_CACHE_HOME="$work/cache" in_profile lock) 2>"$work/err" || status=$?
[ "$status" = 1 ] || fail "in_profile exited $status, not 1, when the profile could not be removed"
grep -q "$work/cache/chromium-headless/expires-" "$work/err" || fail "the removal failure does not name the profile: $(cat "$work/err")"

# A root that cannot be made: refused naming it, the command not run.
touch "$work/file"
rm -f "$work/seen"
status=0
XDG_CACHE_HOME="$work/file" in_profile look 0 2>"$work/err" || status=$?
[ "$status" != 0 ] || fail "in_profile ran with a root that cannot be made"
[ ! -e "$work/seen" ] || fail "in_profile ran the command with no profile"
grep -q "$work/file/chromium-headless" "$work/err" || fail "the refusal does not name the root: $(cat "$work/err")"

# One retention, two places: browser.sh and the Rust launcher.
rust=$(sed -n 's/^const RETENTION_DAYS: u64 = \([0-9]*\);$/\1/p' "$here/../crates/text/src/comparison/chromium_profile.rs")
[ -n "$rust" ] && [ "$rust" = "$PROFILE_RETENTION_DAYS" ] || fail "PROFILE_RETENTION_DAYS ($PROFILE_RETENTION_DAYS, scripts/browser.sh) differs from RETENTION_DAYS (${rust:-not found}, crates/text/src/comparison/chromium_profile.rs)"

# Every headless launch in the page checks is in a function that passes
# --user-data-dir="$profile" and is only ever called through in_profile.
for script in check-page-contract.sh web-idle-frames.sh web-wake-frames.sh; do
  launches=$(awk '/^[a-z_]+\(\) \{/ { fn = $1; sub(/\(\)/, "", fn) } /^\}/ { fn = "" }
    /"\$chrome" --headless/ { print (fn == "" ? "-" : fn) }' "$here/$script" | sort -u)
  [ -n "$launches" ] || fail "$script: no headless launch found"
  for fn in $launches; do
    [ "$fn" != - ] || { fail "$script: a headless launch outside a function"; continue; }
    body=$(awk -v fn="$fn" '$0 ~ "^" fn "\\(\\) \\{" { on = 1 } on { print } on && /^\}/ { exit }' "$here/$script")
    grep -qF -- '--user-data-dir="$profile"' <<<"$body" || fail "$script: $fn launches the browser without --user-data-dir=\"\$profile\""
    calls=$(grep -nwE -- "$fn" "$here/$script" | grep -vE "^[0-9]+:$fn\(\) \{" | grep -vE "^[0-9]+:\s*#" || true)
    [ -n "$calls" ] || fail "$script: $fn is never called"
    while IFS= read -r call; do
      [ -z "$call" ] || grep -qE "in_profile $fn( |$)" <<<"$call" || fail "$script: $fn called without in_profile: $call"
    done <<<"$calls"
  done
done

exit "$failed"
