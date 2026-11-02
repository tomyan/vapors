#!/bin/sh
# The built page's text input against Chromium's own <input> (E6): the
# same DevTools input calls (typed text, an IME's composition and its
# commit, keys, a double click, a paste) on a plain <input> and on the page's field demo, the
# text and selection compared after each (the web_input_oracle example).
#
# Usage: scripts/web-text-input.sh target/web <new or empty dir>
# after `cargo xtask web`. Needs python3 and a browser (scripts/
# browser.sh picks it and its SwiftShader driver, as the page contract
# does). Exits 1 naming each step that differed, with the page's
# console.
# The directory is removed when the check passes and kept, for its logs,
# when it fails.
set -eu
web=${1:?usage: scripts/web-text-input.sh target/web <new or empty dir>}
work=${2:?usage: scripts/web-text-input.sh target/web <new or empty dir>}
. "$(dirname "$0")/browser.sh"
[ -f "$web/index.html" ] || { echo "$web/index.html: missing: the web job failed, or its build was not downloaded, so the web input check was not run; fix: the web job (in CI) or cargo xtask web (locally)" >&2; exit 1; }
if [ -e "$work" ] && [ -n "$(ls -A "$work")" ]; then echo "$work: not empty" >&2; exit 1; fi
mkdir -p "$work/site" "$work/profile"; cp "$web"/* "$work/site"/
# Chromium leaves org.chromium.Chromium.* directories in TMPDIR on every
# launch; here they go with $work.
mkdir -p "$work/tmp"; export TMPDIR="$work/tmp"
printf '<!doctype html><meta charset="utf-8"><input id="i" value="Warm">\n' > "$work/site/input.html"

# The whole run: a cold Chrome's 58 s to the page (COLD_START in
# crates/text/src/comparison/chromium_warm.rs), the example's 60 s for
# the page's first frame and mirror, and the steps: 240 s.
RUN_S=240

(cd "$work/site" && exec python3 -c "
import http.server
class Quiet(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *args):
        pass
server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Quiet)
open('../port', 'w').write(str(server.server_address[1]))
server.serve_forever()") >/dev/null 2>&1 &
server=$!
# on every exit: the server now, the browser once it has started
browser_pid=
trap 'kill $server 2>/dev/null; [ -z "$browser_pid" ] || kill -- -$browser_pid 2>/dev/null; true' EXIT
chrome=$(browser)
version=$("$chrome" --version 2>/dev/null) && [ -n "$version" ] || { echo "$chrome --version failed or printed nothing; fix: check the browser starts" >&2; exit 1; }
VK_DRIVER_FILES=$(vulkan_driver "$chrome" "$version")
export VK_DRIVER_FILES
# its own process group, killed whole: `chromium` is a wrapper
setsid env -u DBUS_SESSION_BUS_ADDRESS "$chrome" --headless=new --no-sandbox \
  --disable-gpu-sandbox --use-angle=swiftshader --enable-unsafe-webgpu --enable-features=Vulkan,WebGPU \
  --window-size=640,400 --enable-logging=stderr --v=0 --remote-debugging-port=0 \
  --user-data-dir="$work/profile" about:blank 2>"$work/chrome.log" &
browser_pid=$!
# the server wrote its port 45 ms after starting (five runs on the dev
# box at load 14, 2026-11-01); 10 s allows a cold runner's Python
for _ in $(seq 100); do [ -s "$work/port" ] && break; sleep 0.1; done
[ -s "$work/port" ] || { echo "the page server did not start within 10 s" >&2; exit 1; }
if timeout "$RUN_S" cargo run -q -p vapors-elements-corpus --example web_input_oracle -- "$work/profile" "http://127.0.0.1:$(cat "$work/port")"; then
  # the browser's group gone before its profile is removed: a process
  # still writing there would leave files behind
  kill -- -"$browser_pid" 2>/dev/null || true
  for _ in $(seq 100); do pgrep -g "$browser_pid" >/dev/null || break; sleep 0.1; done
  pgrep -g "$browser_pid" >/dev/null && { echo "the browser's processes outlived 10 s after the kill; $work kept" >&2; exit 1; }
  browser_pid=
  rm -rf "$work"
  exit 0
fi
echo "  ($version) the page's console and Chrome's last lines:" >&2
grep -E 'CONSOLE' "$work/chrome.log" | tail -10 | sed 's#^#    #' >&2
grep -v CONSOLE "$work/chrome.log" | tail -10 | sed 's#^#    #' >&2
exit 1
