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
# The removal-failure case relies on a directory mode of 500 stopping its
# owner deleting inside it, which root ignores.
[ "$(id -u)" != 0 ] || { echo "FAIL run as a normal user, not root: root removes a mode-500 directory, so the removal-failure case cannot be made; reproduce: id -u" >&2; exit 1; }
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
grep -q XDG_CACHE_HOME "$work/err" && grep -q "nor HOME" "$work/err" || fail "the refusal does not name XDG_CACHE_HOME and HOME: $(cat "$work/err")"

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

# A process of the launch that outlives it (Chromium's children carry
# --user-data-dir=<profile>): waited for up to PROFILE_SETTLE_S, then
# killed and named, before the profile is removed.
linger() { (exec -a "fake-chromium --user-data-dir=$profile" sleep "$1") & echo $! >"$work/linger.pid"; }
PROFILE_SETTLE_S=0.5 XDG_CACHE_HOME="$work/settle" in_profile linger 0.2 2>"$work/err" || fail "in_profile failed for a child that ended within the deadline: $(cat "$work/err")"
[ ! -s "$work/err" ] || fail "in_profile reported a child that ended within the deadline: $(cat "$work/err")"
status=0
PROFILE_SETTLE_S=0.3 XDG_CACHE_HOME="$work/settle" in_profile linger 30 2>"$work/err" || status=$?
pid=$(cat "$work/linger.pid")
sleep 0.1
if kill -0 "$pid" 2>/dev/null; then fail "in_profile left process $pid, still carrying its profile, running"; kill "$pid"; fi
grep -q "still running" "$work/err" || fail "in_profile did not name the process it killed: $(cat "$work/err")"
[ "$status" = 1 ] || fail "in_profile returned $status, not 1, after killing a leftover process of a command that exited 0"
[ -z "$(ls "$work/settle/chromium-headless")" ] || fail "in_profile left a profile: $(ls "$work/settle/chromium-headless")"

# A profile path with regex metacharacters is still matched literally.
status=0
PROFILE_SETTLE_S=0.3 XDG_CACHE_HOME="$work/c[a]che+" in_profile linger 30 2>"$work/err" || status=$?
pid=$(cat "$work/linger.pid")
sleep 0.1
if kill -0 "$pid" 2>/dev/null; then fail "in_profile missed process $pid under a profile path with [ ] +"; kill "$pid"; fi
[ "$status" = 1 ] || fail "in_profile returned $status under a profile path with [ ] +: $(cat "$work/err")"

# A pgrep that fails (status 2) stops the shell in_profile runs in; it is
# never read as "no process left".
mkdir -p "$work/bin"
printf '#!/bin/sh\necho "pgrep: broken (fake)" >&2\nexit 2\n' >"$work/bin/pgrep"
chmod 755 "$work/bin/pgrep"
status=0
(PATH="$work/bin:$PATH" XDG_CACHE_HOME="$work/broken" in_profile true; echo "went on" >"$work/went-on") 2>"$work/err" || status=$?
[ "$status" = 1 ] || fail "in_profile exited $status, not 1, when pgrep failed: $(cat "$work/err")"
[ ! -e "$work/went-on" ] || fail "the shell went on after pgrep failed"
grep -q "pgrep failed" "$work/err" || fail "the pgrep failure is not reported: $(cat "$work/err")"

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

# Every headless browser launch in scripts/*.sh is in a function that
# passes --user-data-dir="$profile" in that command and is only ever
# called through in_profile. A command is a line with its backslash
# continuations joined and a trailing " # ..." comment removed, split at
# ; && || and |; a launch is a command naming --headless.
# Limits: only *.sh is scanned (scripts/chromium-focus-fixup.mjs keeps its
# own $work/profile until slice 6, like web-text-input.sh below); a
# function ends at a "}" in column one, so one closed by an indented "}"
# is blamed for the next top-level launch (still a failure).
# A quoted argument holding "|", ";", "&&" or " #" splits a correct launch,
# which is then reported as missing the flag (a false failure, never a
# false pass); none of today's launches has one.
# Relaxed: web-text-input.sh keeps its own $work/profile until slice 6 of
# the expiry-profile plan (PLAN in the retention project) moves it;
# remove it from this list then.
relaxed=" web-text-input.sh "
for script in "$here"/*.sh; do
  name=$(basename "$script")
  case $relaxed in *" $name "*) continue ;; esac
  [ "$name" != browser-profile-check.sh ] || continue
  awk -v file="$name" '
    /^[A-Za-z_][A-Za-z0-9_]*\(\) \{/ { fn = $1; sub(/\(\).*/, "", fn) }
    { line = (cont ? line " " : "") $0; start = cont ? start : NR; cont = /\\$/ }
    !cont {
      if (line !~ /^[ \t]*#/) {
        sub(/[ \t]+#.*$/, "", line)
        n = split(line, cmds, /;|&&|\|\||\|/)
        for (i = 1; i <= n; i++) if (cmds[i] ~ /--headless/)
          print file ":" start ":" (fn == "" ? "-" : fn) ":" (index(cmds[i], "--user-data-dir=\"$profile\"") ? "flag" : "none")
      }
      line = ""
    }
    /^\}/ { fn = "" }' "$script"
done >"$work/launches"
while IFS=: read -r name line fn flag; do
  [ "$fn" != - ] || { fail "$name:$line: a headless launch outside a function"; continue; }
  [ "$flag" = flag ] || fail "$name:$line: $fn launches the browser without --user-data-dir=\"\$profile\" in that command"
  calls=$(grep -nwE -- "$fn" "$here/$name" | grep -vE "^[0-9]+:$fn\(\) \{" | grep -vE "^[0-9]+:\s*#" || true)
  [ -n "$calls" ] || fail "$name: $fn is never called"
  while IFS= read -r call; do
    [ -z "$call" ] || grep -qE "in_profile $fn( |$)" <<<"$call" || fail "$name: $fn called without in_profile: $call"
  done <<<"$calls"
done <"$work/launches"
[ -s "$work/launches" ] || fail "no headless launch found in scripts/*.sh; the scan is broken"

exit "$failed"
