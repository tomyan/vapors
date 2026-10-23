#!/bin/sh
# Frames the built page draws while nothing happens, per demo, in
# headless Chromium in real time (no virtual time): the page loads,
# waits 3 s for start-up, then counts getCurrentTexture calls (one per
# frame) over the next 3 s. An app that redraws only on change draws 0.
#
# A page that drew no frame by 3 s, or showed an error, is refused: an
# idle count of 0 from a page that never drew would be a false pass.
#
# Usage: scripts/web-idle-frames.sh target/web <new or empty dir> [demo ...]
# after `cargo xtask web`; demos default to hello counter image.
# Needs python3 and chromium (or $CHROME). Prints one line per demo and
# exits 1 if a page never reported (its count is missing).
set -eu
web=${1:?usage: scripts/web-idle-frames.sh target/web <new or empty dir> [demo ...]}
work=${2:?usage: scripts/web-idle-frames.sh target/web <new or empty dir> [demo ...]}
shift 2
demos=${*:-hello counter image}
chrome=${CHROME:-chromium}
[ -f "$web/index.html" ] || { echo "$web/index.html: missing; run cargo xtask web first" >&2; exit 1; }
if [ -e "$work" ] && [ -n "$(ls -A "$work")" ]; then echo "$work: not empty" >&2; exit 1; fi
mkdir -p "$work"; cp "$web"/* "$work"/
python3 - "$work/index.html" "$work/idle.html" <<'PY'
import sys
source, target = sys.argv[1:3]
text = open(source).read()
head = ('<script>var frames = 0, start; var current = GPUCanvasContext.prototype.getCurrentTexture;'
        ' GPUCanvasContext.prototype.getCurrentTexture = function () { frames++; return current.apply(this, arguments); };'
        ' var refused = "";'
        ' setTimeout(function () { start = frames; var e = document.getElementById("error");'
        ' if (frames === 0) refused = "the page drew no frame in 3 s";'
        ' else if (e && e.textContent) refused = "the page reported an error: " + e.textContent; }, 3000);'
        ' setTimeout(function () { var p = document.createElement("pre"); p.id = "idle";'
        ' p.textContent = refused ? "refused: " + refused : String(frames - start);'
        ' document.body.append(p); fetch("/done", { method: "POST" }); }, 6000);</script>')
open(target, 'w').write(text.replace('</head>', head + '</head>').replace('</body>', '<img src="/wait" hidden></body>'))
PY
# The page's load waits on /wait until it posts /done (at most 20 s), so
# the dump comes after the count.
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
# A free port, chosen by the server and written to $work/port; 10 s to start.
for _ in $(seq 100); do [ -s "$work/port" ] && break; sleep 0.1; done
[ -s "$work/port" ] || { echo "the page server did not start within 10 s" >&2; exit 1; }
port=$(cat "$work/port")
icd=$(dirname "$(readlink -f "$(command -v "$chrome")")")/vk_swiftshader_icd.json
[ -r "$icd" ] || icd=/usr/lib/chromium/vk_swiftshader_icd.json
missing=0
for demo in $demos; do
  count=$(env -u DBUS_SESSION_BUS_ADDRESS VK_DRIVER_FILES="$icd" timeout 60 "$chrome" --headless=new --no-sandbox \
    --disable-gpu-sandbox --use-angle=swiftshader --enable-unsafe-webgpu --enable-features=Vulkan,WebGPU \
    --window-size=640,400 --dump-dom "http://127.0.0.1:$port/idle.html?demo=$demo" 2>/dev/null \
    | sed -n 's#.*<pre id="idle">\([^<]*\)</pre>.*#\1#p')
  case $count in
    '') echo "$demo: no count; the page never reported" >&2; missing=1; continue ;;
    refused:*) echo "$demo: $count" >&2; missing=1; continue ;;
  esac
  echo "$demo: $count frames in 3 s idle"
done
exit $missing
