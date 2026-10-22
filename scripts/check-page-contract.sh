#!/bin/sh
# The page contract on the built web demo, in headless Chrome: an error
# lands in #error when the page has one and in #status when it has not;
# a status line never lands in #error. Four copies of the built page,
# each checked; any mismatch fails naming the case.
#
# Usage: scripts/check-page-contract.sh target/web <new or empty dir>
# after `cargo xtask web`. CI runs it in the `web` job. Needs python3
# (the server) and a browser: $CHROME, else google-chrome, else
# chromium.
set -eu
web=${1:?usage: scripts/check-page-contract.sh target/web <new or empty dir>}
work=${2:?usage: scripts/check-page-contract.sh target/web <new or empty dir>}
port=8731
failed=0

fail() { echo "FAIL $*" >&2; failed=1; }

browser() {
  for candidate in "${CHROME:-}" google-chrome chromium; do
    if [ -n "$candidate" ] && command -v "$candidate" >/dev/null 2>&1; then
      echo "$candidate"; return
    fi
  done
  echo "no browser: set CHROME, or install google-chrome or chromium" >&2
  exit 1
}

prepare() {
  [ -f "$web/index.html" ] || { echo "$web/index.html: missing; run cargo xtask web first" >&2; exit 1; }
  if [ -e "$work" ] && [ -n "$(ls -A "$work")" ]; then
    echo "$work: not empty; give a new or empty directory" >&2; exit 1
  fi
  mkdir -p "$work"; cp "$web"/* "$work"/
  nogpu='<script>delete Navigator.prototype.gpu;</script>'
  cp "$work/index.html" "$work/as-built.html"
  sed 's#<button id="open">Open</button>##' "$work/index.html" > "$work/no-open-button.html"
  sed "s#</head>#$nogpu</head>#" "$work/index.html" > "$work/no-webgpu.html"
  sed "s#</head>#$nogpu</head>#; s#<p id=\"error\"[^>]*></p>##" "$work/index.html" \
    > "$work/no-webgpu-no-error-element.html"
  grep -q '<p id="error"' "$work/as-built.html" || { echo "$web/index.html: no <p id=\"error\">" >&2; exit 1; }
}

# The server answers within 10 s or the check fails (it starts in well
# under 1 s here).
serve() {
  (cd "$work" && exec python3 -m http.server "$port" --bind 127.0.0.1) >/dev/null 2>&1 &
  server=$!
  trap 'kill $server' EXIT
  python3 -c "
import sys, time, urllib.request
deadline = time.time() + 10
while time.time() < deadline:
    try:
        urllib.request.urlopen('http://127.0.0.1:$port/index.html', timeout=1); sys.exit(0)
    except OSError:
        time.sleep(0.1)
sys.exit('the page server on port $port did not answer within 10 s')"
}

# One page's DOM after 8 s of virtual time for the first frame
# (SwiftShader rasterises on the CPU). 60 s is the deadline: Chromium
# 155 here dumps a page in under 2 s. No session bus: with one and a portal
# that cannot start, every load waits 120 s (docs/web.md).
dump() {
  env -u DBUS_SESSION_BUS_ADDRESS timeout 60 "$chrome" --headless=new --no-sandbox \
    --disable-gpu-sandbox --use-angle=swiftshader --enable-unsafe-webgpu \
    --enable-features=Vulkan,WebGPU --window-size=640,400 \
    --virtual-time-budget=8000 --dump-dom "http://127.0.0.1:$port/$1.html" 2>/dev/null
}

# check <page> <#status pattern> <#error: absent | hidden | pattern>
check() {
  dom=$(dump "$1") || { fail "$1: $chrome exited non-zero or passed 60 s"; return; }
  status=$(printf '%s' "$dom" | grep -o '<p id="status">[^<]*</p>' | sed 's#<[^>]*>##g')
  error_tag=$(printf '%s' "$dom" | grep -o '<p id="error"[^>]*>[^<]*</p>' || true)
  error=$(printf '%s' "$error_tag" | sed 's#<[^>]*>##g')
  echo "$1: #status [$status] #error [${error_tag:-absent}]"
  printf '%s' "$status" | grep -Eq "$2" || fail "$1: #status is [$status], wanted /$2/"
  case $3 in
    absent) [ -z "$error_tag" ] || fail "$1: #error exists, wanted none" ;;
    hidden) printf '%s' "$error_tag" | grep -q ' hidden' && [ -z "$error" ] \
              || fail "$1: #error is [$error_tag], wanted hidden and empty" ;;
    *) printf '%s' "$error_tag" | grep -q ' hidden' && fail "$1: #error is still hidden"
       printf '%s' "$error" | grep -Eq "$3" || fail "$1: #error is [$error], wanted /$3/" ;;
  esac
}

chrome=$(browser)
echo "browser: $chrome ($("$chrome" --version 2>/dev/null || echo 'version unknown'))"
prepare
serve
check as-built '^adapter: ' hidden
check no-open-button '^adapter: ' 'touch screens cannot open files'
check no-webgpu '^starting$' '^no WebGPU adapter'
check no-webgpu-no-error-element '^no WebGPU adapter' absent
[ "$failed" = 0 ] || { echo "the page contract does not hold; reproduce: cargo xtask web && $0 target/web \$(mktemp -d)" >&2; exit 1; }
echo "ok   the page contract holds in all four cases"
