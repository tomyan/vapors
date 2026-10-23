#!/bin/sh
# The waker on the built page (?demo=wake) in headless Chrome in real
# time: once the page has drawn, no frame in 2 s idle, then exactly one
# frame for each of 3 calls to vapors_demo_wake, 300 ms apart, and one
# or two for a burst of 5 calls with no wait between (frames counted
# with getCurrentTexture).
#
# Usage: scripts/web-wake-frames.sh target/web <new or empty dir>
# after `cargo xtask web`. Needs python3 and a browser (scripts/
# browser.sh picks it and its SwiftShader driver, as the page contract
# does). Prints the counts; exits 1 naming what differed, or the last
# stage the page reached and its console when it never reported.
#
# The page posts each stage as it reaches it: script (its first script
# ran), drew (the first frame), export (vapors_demo_wake is there),
# counts. In CI it runs after the page contract, so Chrome is warm.
set -eu
web=${1:?usage: scripts/web-wake-frames.sh target/web <new or empty dir>}
work=${2:?usage: scripts/web-wake-frames.sh target/web <new or empty dir>}
. "$(dirname "$0")/browser.sh"
[ -f "$web/index.html" ] || { echo "$web/index.html: missing; run cargo xtask web first" >&2; exit 1; }
if [ -e "$work" ] && [ -n "$(ls -A "$work")" ]; then echo "$work: not empty" >&2; exit 1; fi
mkdir -p "$work"; cp "$web"/* "$work"/

# The first frame: the page contract's wasm pages draw within its 8 s of
# virtual time; a cold runner's first wasm compile is slower, and the
# gpu job's cold Chrome took up to 57.9 s to its first page (COLD_START
# in crates/text/src/comparison/chromium_warm.rs). 30 s from the page's
# own script, then the counts take about 3 s.
FIRST_FRAME_MS=30000
# The whole run: a cold Chrome's 58 s to the page, the 30 s for the first
# frame, the counts, and margin: 120 s.
RUN_S=120

python3 - "$work/index.html" "$work/wake.html" "$FIRST_FRAME_MS" <<'PY'
import sys
source, target, first_ms = sys.argv[1:4]
text = open(source).read()
head = ('<script>var frames = 0; var current = GPUCanvasContext.prototype.getCurrentTexture;'
        ' GPUCanvasContext.prototype.getCurrentTexture = function () { frames++; return current.apply(this, arguments); };'
        ' function wait(ms) { return new Promise(function (r) { setTimeout(r, ms); }); }'
        ' function stage(name) { return fetch("/stage/" + name, { method: "POST" }).catch(function () {}); }'
        ' function report(text) { var p = document.createElement("pre"); p.id = "wake"; p.textContent = text;'
        ' document.body.append(p); stage("counts").then(function () { fetch("/done", { method: "POST" }); }); }'
        ' stage("script");'
        ' (async function () { var start = performance.now();'
        ' while (frames === 0 && performance.now() - start < ' + first_ms + ') { await wait(100); }'
        ' var e = document.getElementById("error");'
        ' if (frames === 0) { report("refused: the page drew no frame in ' + str(int(first_ms) // 1000) + ' s"); return; }'
        ' stage("drew"); await wait(500);'
        ' if (e && e.textContent) { report("refused: the page reported an error: " + e.textContent); return; }'
        ' if (!window.wasmBindings || !window.wasmBindings.vapors_demo_wake) { report("refused: no vapors_demo_wake export"); return; }'
        ' stage("export");'
        ' var at0 = frames; await wait(2000); var idle = frames - at0; var each = [];'
        ' for (var i = 0; i < 3; i++) { var at = frames;'
        ' try { window.wasmBindings.vapors_demo_wake(); } catch (x) { report("refused: vapors_demo_wake threw " + x); return; }'
        ' await wait(300); each.push(frames - at); }'
        ' var before = frames;'
        ' try { for (var j = 0; j < 5; j++) { window.wasmBindings.vapors_demo_wake(); } } catch (x) { report("refused: vapors_demo_wake threw " + x); return; }'
        ' await wait(300); var burst = frames - before;'
        ' report("idle " + idle + "; per wake " + each.join(",") + "; burst " + (burst >= 1 && burst <= 2 ? "1-2" : burst)); })();</script>')
open(target, 'w').write(text.replace('</head>', head + '</head>').replace('</body>', '<img src="/wait" hidden></body>'))
PY
# The page's load waits on /wait until it posts /done, so the dump comes
# after the counts; at most the run's own deadline. Each stage the page
# posts is appended to $work/stages; each request for the page, too.
(cd "$work" && exec python3 -c "
import http.server, threading
done = threading.Event()
def note(line):
    with open('stages', 'a') as out:
        out.write(line + '\n')
class Pages(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        if self.path.startswith('/wake.html'):
            note('page requested')
        if self.path != '/wait':
            return super().do_GET()
        done.wait($RUN_S); done.clear(); self.send_response(204); self.end_headers()
    def do_POST(self):
        if self.path.startswith('/stage/'):
            note(self.path[len('/stage/'):])
        else:
            done.set()
        self.send_response(204); self.end_headers()
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
started=$(date +%s)
counts=$(env -u DBUS_SESSION_BUS_ADDRESS timeout "$RUN_S" "$chrome" --headless=new --no-sandbox \
  --disable-gpu-sandbox --use-angle=swiftshader --enable-unsafe-webgpu --enable-features=Vulkan,WebGPU \
  --window-size=640,400 --enable-logging=stderr --v=0 --dump-dom "http://127.0.0.1:$port/wake.html?demo=wake" 2>"$work/chrome.log" \
  | sed -n 's#.*<pre id="wake">\([^<]*\)</pre>.*#\1#p') || true
took=$(( $(date +%s) - started ))
case $counts in
  "idle 0; per wake 1,1,1; burst 1-2") echo "wake: $counts ($version, ${took} s)"; exit 0 ;;
  '') echo "wake: no counts after ${took} s ($version, deadline ${RUN_S} s); the stages the page reached: $(if [ -f "$work/stages" ]; then tr '\n' ' ' < "$work/stages"; else echo 'none, not even the request for the page'; fi)" >&2 ;;
  *) echo "wake: $counts; expected idle 0; per wake 1,1,1; burst 1-2" >&2 ;;
esac
echo "  the page's console and Chrome's last lines:" >&2
grep -E 'CONSOLE' "$work/chrome.log" | tail -10 | sed 's#^#    #' >&2
grep -v CONSOLE "$work/chrome.log" | tail -10 | sed 's#^#    #' >&2
exit 1
