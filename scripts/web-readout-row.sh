#!/bin/sh
# The GPU-time readout's row on a phone-width page (#491, L69): one-scene
# and the racer at ?samples=4 in headless Chrome at 412 x 915. The
# readout (#demo-readout, crates/web-demo/index.html) is shown, wrapped
# in full (not clipped, inside the bar), and the canvas runs from the
# bar's bottom to the window's. One-scene draws every frame, so its
# line carries a reading; the racer, paused, draws none after its
# first frames, so its line is the waiting one.
#
# Usage: scripts/web-readout-row.sh target/web <new or empty dir> after
# `cargo xtask web`. Needs python3 and a browser (scripts/browser.sh).
# Prints each demo's line; exits 1 naming what differed. The directory
# is removed when the check passes and kept, for its logs, when it fails.
set -eu
web=${1:?usage: scripts/web-readout-row.sh target/web <new or empty dir>}
work=${2:?usage: scripts/web-readout-row.sh target/web <new or empty dir>}
. "$(dirname "$0")/browser.sh"
[ -f "$web/index.html" ] || { echo "$web/index.html: missing; run cargo xtask web first" >&2; exit 1; }
if [ -e "$work" ] && [ -n "$(ls -A "$work")" ]; then echo "$work: not empty" >&2; exit 1; fi
mkdir -p "$work"; cp "$web"/* "$work"/
mkdir -p "$work/tmp"; export TMPDIR="$work/tmp"

# A reading needs the first frames, then a second of them: a cold
# Chrome took up to 57.9 s to its first page (web-wake-frames.sh), so
# 45 s from the page's own script, and 120 s in all.
READING_MS=45000
RUN_S=120

python3 - "$work/index.html" "$work/readout.html" "$READING_MS" <<'PY'
import sys
source, target, reading_ms = sys.argv[1:4]
text = open(source).read()
head = ('<script>function wait(ms) { return new Promise(function (r) { setTimeout(r, ms); }); }'
        ' function report(text) { var p = document.createElement("pre"); p.id = "readout-check"; p.textContent = text;'
        ' document.body.append(p); fetch("/done", { method: "POST" }); }'
        ' (async function () { var start = performance.now();'
        ' var reading = /demo=one-scene/.test(location.search) ? /% of|frame interval/ : /GPU/;'
        ' var r = null; while (performance.now() - start < ' + reading_ms + ') {'
        ' r = document.getElementById("demo-readout"); if (r && reading.test(r.textContent)) { break; } await wait(200); }'
        ' if (!r || !reading.test(r.textContent)) { report("refused: no readout line in ' + str(int(reading_ms) // 1000) + ' s: " + (r ? r.textContent : "no #demo-readout")); return; }'
        ' await wait(500);'
        ' var bar = document.getElementById("demo-bar"), canvas = document.querySelector("canvas");'
        ' var rb = r.getBoundingClientRect(), bb = bar.getBoundingClientRect(), cb = canvas.getBoundingClientRect();'
        ' var clipped = r.hidden || r.scrollWidth > r.clientWidth || rb.right > innerWidth + 0.5 || rb.bottom > bb.bottom + 0.5;'
        ' var fills = Math.abs(cb.top - bb.bottom) < 1 && Math.abs(cb.bottom - innerHeight) < 1;'
        ' report((clipped ? "clipped" : "shown") + "; " + (fills ? "canvas fills" : "canvas " + cb.top + " to " + cb.bottom + " under a bar of " + bb.bottom) + "; " + r.textContent); })();</script>')
open(target, 'w').write(text.replace('</head>', head + '</head>').replace('</body>', '<img src="/wait" hidden></body>'))
PY
(cd "$work" && exec python3 -c "
import http.server, threading
done = threading.Event()
class Pages(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        if self.path != '/wait':
            return super().do_GET()
        done.wait($RUN_S); done.clear(); self.send_response(204); self.end_headers()
    def do_POST(self):
        done.set(); self.send_response(204); self.end_headers()
    def log_message(self, *args):
        pass
server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Pages)
open('port', 'w').write(str(server.server_address[1]))
server.serve_forever()") >/dev/null 2>&1 &
server=$!
trap 'kill $server' EXIT
for _ in $(seq 100); do [ -s "$work/port" ] && break; sleep 0.1; done
[ -s "$work/port" ] || { echo "the page server did not start within 10 s" >&2; exit 1; }
port=$(cat "$work/port")
chrome=$(browser)
version=$("$chrome" --version 2>/dev/null) && [ -n "$version" ] || { echo "$chrome --version failed or printed nothing; fix: check the browser starts" >&2; exit 1; }
VK_DRIVER_FILES=$(vulkan_driver "$chrome" "$version")
export VK_DRIVER_FILES
failed=0
for demo in one-scene racer; do
  line=$(env -u DBUS_SESSION_BUS_ADDRESS timeout "$RUN_S" "$chrome" --headless=new --no-sandbox \
    --disable-gpu-sandbox --use-angle=swiftshader --enable-unsafe-webgpu --enable-features=Vulkan,WebGPU \
    --window-size=412,915 --enable-logging=stderr --v=0 --dump-dom "http://127.0.0.1:$port/readout.html?demo=$demo&samples=4" \
    2>"$work/chrome-$demo.log" | sed -n 's#.*<pre id="readout-check">\([^<]*\)</pre>.*#\1#p') || true
  case $line in
    "shown; canvas fills; "*) echo "readout row, $demo: $line" ;;
    '') echo "readout row, $demo: no result in ${RUN_S} s ($version); Chrome's last lines:" >&2; tail -5 "$work/chrome-$demo.log" | sed 's#^#    #' >&2; failed=1 ;;
    *) echo "readout row, $demo: $line; expected the row shown in full and the canvas filling the rest" >&2; failed=1 ;;
  esac
done
[ "$failed" = 0 ] || { echo "  logs kept in $work" >&2; exit 1; }
rm -rf "$work"
