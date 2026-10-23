#!/bin/sh
# The waker on the built page (?demo=wake) in headless Chromium in real
# time: after 3 s of start-up the page must have drawn, then draw no
# frame in 2 s idle, then exactly one frame for each of 3 calls to
# vapors_demo_wake, 300 ms apart (frames counted with getCurrentTexture).
#
# Usage: scripts/web-wake-frames.sh target/web <new or empty dir>
# after `cargo xtask web`. Needs python3 and chromium (or $CHROME).
# Prints the counts; exits 1 naming what differed, or when the page
# never reported.
set -eu
web=${1:?usage: scripts/web-wake-frames.sh target/web <new or empty dir>}
work=${2:?usage: scripts/web-wake-frames.sh target/web <new or empty dir>}
chrome=${CHROME:-chromium}
[ -f "$web/index.html" ] || { echo "$web/index.html: missing; run cargo xtask web first" >&2; exit 1; }
if [ -e "$work" ] && [ -n "$(ls -A "$work")" ]; then echo "$work: not empty" >&2; exit 1; fi
mkdir -p "$work"; cp "$web"/* "$work"/
python3 - "$work/index.html" "$work/wake.html" <<'PY'
import sys
source, target = sys.argv[1:3]
text = open(source).read()
head = ('<script>var frames = 0; var current = GPUCanvasContext.prototype.getCurrentTexture;'
        ' GPUCanvasContext.prototype.getCurrentTexture = function () { frames++; return current.apply(this, arguments); };'
        ' function wait(ms) { return new Promise(function (r) { setTimeout(r, ms); }); }'
        ' function report(text) { var p = document.createElement("pre"); p.id = "wake"; p.textContent = text;'
        ' document.body.append(p); fetch("/done", { method: "POST" }); }'
        ' (async function () { await wait(3000); var e = document.getElementById("error");'
        ' if (frames === 0) { report("refused: the page drew no frame in 3 s"); return; }'
        ' if (e && e.textContent) { report("refused: the page reported an error: " + e.textContent); return; }'
        ' var start = frames; await wait(2000); var idle = frames - start; var each = [];'
        ' for (var i = 0; i < 3; i++) { var at = frames;'
        ' try { window.wasmBindings.vapors_demo_wake(); } catch (x) { report("refused: vapors_demo_wake threw " + x); return; }'
        ' await wait(300); each.push(frames - at); }'
        ' report("idle " + idle + "; per wake " + each.join(",")); })();</script>')
open(target, 'w').write(text.replace('</head>', head + '</head>').replace('</body>', '<img src="/wait" hidden></body>'))
PY
# The page's load waits on /wait until it posts /done (at most 20 s), so
# the dump comes after the counts.
(cd "$work" && exec python3 -c "
import http.server, threading
done = threading.Event()
class Pages(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        if self.path != '/wait':
            return super().do_GET()
        done.wait(20); done.clear(); self.send_response(204); self.end_headers()
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
icd=$(dirname "$(readlink -f "$(command -v "$chrome")")")/vk_swiftshader_icd.json
[ -r "$icd" ] || icd=/usr/lib/chromium/vk_swiftshader_icd.json
counts=$(env -u DBUS_SESSION_BUS_ADDRESS VK_DRIVER_FILES="$icd" timeout 60 "$chrome" --headless=new --no-sandbox \
  --disable-gpu-sandbox --use-angle=swiftshader --enable-unsafe-webgpu --enable-features=Vulkan,WebGPU \
  --window-size=640,400 --dump-dom "http://127.0.0.1:$port/wake.html?demo=wake" 2>/dev/null \
  | sed -n 's#.*<pre id="wake">\([^<]*\)</pre>.*#\1#p')
case $counts in
  "idle 0; per wake 1,1,1") echo "wake: $counts" ;;
  '') echo "wake: no counts; the page never reported" >&2; exit 1 ;;
  *) echo "wake: $counts; expected idle 0; per wake 1,1,1" >&2; exit 1 ;;
esac
