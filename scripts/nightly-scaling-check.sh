#!/usr/bin/env bash
# Proves scripts/nightly-scaling.sh's guards with a fake git and gh, no
# network and no build: the network deadline on the fetch and on gh, the
# lock against a second run, and a runner failure's issue quoting the
# last line of its stderr. Exits non-zero naming each guard that failed.
#
# Usage: scripts/nightly-scaling-check.sh
set -euo pipefail
script="$(cd "$(dirname "$0")" && pwd)/nightly-scaling.sh"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/bin"
cat >"$work/bin/git" <<'FAKE'
#!/bin/sh
case " $* " in *" fetch "*)
  [ -n "${FAKE_FETCH_SLEEP:-}" ] && exec sleep "$FAKE_FETCH_SLEEP"
  echo "fatal: unable to access origin (fake fetch, pid $$)" >&2; exit 1 ;;
esac
exec /usr/bin/git "$@"
FAKE
cat >"$work/bin/gh" <<'FAKE'
#!/bin/sh
printf '%s\n' "gh $*" >>"$FAKE_GH_LOG"
[ -n "${FAKE_GH_SLEEP:-}" ] && exec sleep "$FAKE_GH_SLEEP"
exit 0
FAKE
chmod +x "$work/bin/git" "$work/bin/gh"
failed=0

reset() { rm -rf "$work/state"; mkdir -p "$work/state"; : >"$work/gh.log"; }

# The script with the fakes, a 2 s network deadline and extra variables
# "$@", under an outer limit of 20 s: a missing deadline shows as a run
# that reaches it. Sets $elapsed.
run() {
  SECONDS=0
  env PATH="$work/bin:$PATH" NIGHTLY_GH="$work/bin/gh" NIGHTLY_STATE="$work/state" \
    NIGHTLY_NETWORK_DEADLINE=2 FAKE_GH_LOG="$work/gh.log" "$@" \
    timeout 20 "$script" >"$work/out" 2>&1 || true
  elapsed=$SECONDS
}

# Guard $1 holds when $2 is 0; otherwise $3 says what happened.
check() {
  if [ "${2//0/}" = "" ]; then echo "ok   $1"; else echo "FAIL $1: $3"; failed=1; fi
}

held() { if "$@"; then echo 0; else echo 1; fi; }

reset; run FAKE_FETCH_SLEEP=30
check "fetch deadline" "$(held [ "$elapsed" -lt 15 ])" \
  "a hung fetch ran ${elapsed} s"

reset; run FAKE_GH_SLEEP=30
check "gh deadline" "$(held [ "$elapsed" -lt 15 ])" "a hung gh ran ${elapsed} s"

reset; flock "$work/state/lock" sleep 30 & holder=$!
sleep 1
run
kill "$holder" 2>/dev/null || true
check "lock" "$(held grep -q "another nightly run holds" "$work/out")$(held [ ! -s "$work/gh.log" ])" \
  "a second run went on: $(head -c 300 "$work/out")"

# The fetch's error is the runner's last stderr line, written just
# before the trap reads the log; the race is run several times.
missed=0
for _ in 1 2 3 4 5 6 7 8 9 10; do
  reset; run
  grep -q "fatal: unable to access origin (fake fetch" "$work/gh.log" || missed=$((missed + 1))
done
check "quoted error" "$( [ "$missed" = 0 ] && echo 0 || echo 1)" "$missed of 10 issues lacked the fetch's error"

exit "$failed"
