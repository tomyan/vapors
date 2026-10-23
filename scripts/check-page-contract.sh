#!/bin/sh
# The page contract on the built web demo, in headless Chrome: an error
# lands in #error when the page has one and in #status when it has not;
# a status line never lands in #error; a lost GPU device is reported in
# #error with the fix. Five copies of the built page, each checked; any
# mismatch fails naming the case.
#
# Usage: scripts/check-page-contract.sh target/web <new or empty dir>
# after `cargo xtask web`. CI runs it in the `web` job. Needs python3
# (the server) and a browser: $CHROME, else google-chrome, else
# chromium. Vulkan is the browser's own SwiftShader driver, beside it.
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
# Validated paths, one per browser package (2026-10-23):
#   /opt/google/chrome/vk_swiftshader_icd.json  Google Chrome 155.0.8059.39 (the runner)
#   /usr/lib/chromium/vk_swiftshader_icd.json   Chromium 155.0.8059.79 (Debian 13)
# A browser update that moves or drops the file, or the library it names,
# fails here by name.
vulkan_driver() {
  case $(readlink -f "$(command -v "$1")") in
    /opt/google/chrome/*) icd=/opt/google/chrome/vk_swiftshader_icd.json ;;
    *) icd=/usr/lib/chromium/vk_swiftshader_icd.json ;;
  esac
  [ -r "$icd" ] || { echo "$1 ($2): no $icd, so no SwiftShader Vulkan driver; fix: find where this version ships vk_swiftshader_icd.json and update vulkan_driver in $0" >&2; exit 1; }
  name=$(sed -n 's|.*"library_path": *"\./\([^"/]*\)".*|\1|p' "$icd")
  [ -n "$name" ] || { echo "$1 ($2): $icd has no \"library_path\": \"./<file>\" (its shape changed); fix: update vulkan_driver in $0" >&2; exit 1; }
  library=$(dirname "$icd")/$name
  [ -f "$library" ] || { echo "$1 ($2): $icd names a library that is not at $library; fix: update vulkan_driver in $0" >&2; exit 1; }
  echo "$icd"
}

# page <name> <script for <head>> <hold condition> [sed edit]: a copy of
# the built page as <name>.html. With a hold condition (JavaScript over
# `status` and `error`, the two elements) the page's load waits on
# /wait/<name> until the condition holds and the page posts
# /done/<name> (serve below): virtual time does not wait for the GPU
# set-up or the GPU process's answers, but it pauses while a request is
# pending, so the dump comes after what the case asserts, not before.
page() {
  sed "${4:-}" "$work/index.html" > "$work/$1.html"
  python3 - "$work/$1.html" "$1" "$2" "$3" <<'PY'
import sys
path, name, head, condition = sys.argv[1:5]
text = open(path).read()
if condition:
    head += ('<script>addEventListener("DOMContentLoaded", function () {'
             ' var status = document.getElementById("status"), error = document.getElementById("error");'
             ' function met() { return ' + condition + '; }'
             ' function post() { fetch("/done/' + name + '", { method: "POST" }); }'
             ' if (met()) { post(); return; }'
             ' var watch = new MutationObserver(function () { if (met()) { watch.disconnect(); post(); } });'
             ' watch.observe(document.body, { subtree: true, childList: true, characterData: true, attributes: true });'
             ' });</script>')
    text = text.replace('</body>', '<img src="/wait/' + name + '" alt="" hidden></body>')
text = text.replace('</head>', head + '</head>')
open(path, 'w').write(text)
PY
}

prepare() {
  [ -f "$web/index.html" ] || { echo "$web/index.html: missing; run cargo xtask web first" >&2; exit 1; }
  if [ -e "$work" ] && [ -n "$(ls -A "$work")" ]; then
    echo "$work: not empty; give a new or empty directory" >&2; exit 1
  fi
  mkdir -p "$work"; cp "$web"/* "$work"/
  nogpu='<script>delete Navigator.prototype.gpu;</script>'
  # The device is destroyed 1 s after the page gets it, between frames,
  # as a GPU reset does: WebGPU's device.lost resolves with "destroyed".
  # Animation frames stop 500 ms before that, as in a backgrounded tab,
  # so no frame already asked for is left to run: only the loss itself,
  # not a next frame, can report it.
  lose='<script>var request = GPUAdapter.prototype.requestDevice; GPUAdapter.prototype.requestDevice = function () { return request.apply(this, arguments).then(function (device) { setTimeout(function () { Window.prototype.requestAnimationFrame = window.requestAnimationFrame = function () { return 0; }; }, 500); setTimeout(function () { device.destroy(); }, 1000); return device; }); };</script>'
  adapter='status.textContent.indexOf("adapter: ") === 0'
  page as-built "" "$adapter"
  # The screenshot's own copy, held until the canvas has presented:
  # wgpu calls the canvas context's getCurrentTexture once per frame,
  # counted on <body> as data-frames; at the third call the first two
  # frames have presented. Without its own page and hold the screenshot
  # was taken before any frame (vapors main, white centre, after #25).
  frames='<script>var current = GPUCanvasContext.prototype.getCurrentTexture; GPUCanvasContext.prototype.getCurrentTexture = function () { if (document.body) { document.body.dataset.frames = String((Number(document.body.dataset.frames) || 0) + 1); } return current.apply(this, arguments); };</script>'
  page as-built-shot "$frames" "(Number(document.body.dataset.frames) || 0) >= 3"
  page no-open-button "" "!error.hidden && $adapter" 's#<button id="open">Open</button>##'
  page no-webgpu "$nogpu" ""
  page no-webgpu-no-error-element "$nogpu" "" 's#<p id="error"[^>]*></p>##'
  page device-lost "$lose" "!error.hidden"
  grep -q '<p id="error"' "$work/as-built.html" || { echo "$web/index.html: no <p id=\"error\">" >&2; exit 1; }
}

# The server answers within 10 s or the check fails (it starts in well
# under 1 s here). It serves the copies, and holds /wait/<page> until a
# POST to /done/<page> or 30 s. The device-lost page, from Chromium's
# start to its dump with the error shown, took 1.5 s here (Debian 13,
# Chromium 155, 2026-10-23); the runner is several times slower to
# start (touche's check_zoom_status: 17.8 to 44.7 s to a first request,
# runs 39873355321 and 39874643788), so 30 s, under dump's 60 s.
# Each case prints its time, so the runner's are in every CI log.
serve() {
  (cd "$work" && exec python3 -c "
import collections, http.server, threading
flags, lock = collections.defaultdict(threading.Event), threading.Lock()
def flag(path):
    with lock:
        return flags[path.split('/', 2)[2]]
class Pages(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        if not self.path.startswith('/wait/'):
            return super().do_GET()
        flag(self.path).wait(30)
        self.send_response(204); self.end_headers()
    def do_POST(self):
        flag(self.path).set()
        self.send_response(204); self.end_headers()
    def log_message(self, *args):
        pass
http.server.ThreadingHTTPServer(('127.0.0.1', $port), Pages).serve_forever()") >/dev/null 2>&1 &
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
# (SwiftShader rasterises on the CPU), and after any hold; its stderr,
# with the page's console, in $work/<page>.log. 60 s is the deadline:
# Chromium 155 here dumps a page in under 2 s. No session bus: with one
# and a portal that cannot start, every load waits 120 s (docs/web.md).
dump() {
  env -u DBUS_SESSION_BUS_ADDRESS timeout 60 "$chrome" --headless=new --no-sandbox \
    --disable-gpu-sandbox --use-angle=swiftshader --enable-unsafe-webgpu \
    --enable-features=Vulkan,WebGPU --window-size=640,400 --enable-logging=stderr --v=0 \
    --virtual-time-budget=8000 --dump-dom "http://127.0.0.1:$port/$1.html" 2>"$work/$1.log"
}

# The page as Chromium draws it, as $work/<page>.png, after the page's
# own hold; its stderr in $work/<page>.log.
shoot() {
  env -u DBUS_SESSION_BUS_ADDRESS timeout 60 "$chrome" --headless=new --no-sandbox \
    --disable-gpu-sandbox --use-angle=swiftshader --enable-unsafe-webgpu \
    --enable-features=Vulkan,WebGPU --window-size=640,400 --enable-logging=stderr --v=0 \
    --virtual-time-budget=8000 --screenshot="$work/$1.png" "http://127.0.0.1:$port/$1.html" 2>"$work/$1.log"
}

# The r,g,b of the centre pixel of a PNG (8-bit RGB or RGBA, not
# interlaced, as Chromium writes them); anything else is refused by name.
centre_pixel() {
  python3 - "$1" <<'PY'
import struct, sys, zlib
path = sys.argv[1]
data = open(path, 'rb').read()
assert data[:8] == b'\x89PNG\r\n\x1a\n', f'{path}: not a PNG'
chunks, at = {}, 8
while at < len(data):
    length, kind = struct.unpack('>I4s', data[at:at + 8])
    chunks.setdefault(kind, b'')
    chunks[kind] += data[at + 8:at + 8 + length]
    at += 12 + length
width, height, depth, colour, _, _, interlace = struct.unpack('>IIBBBBB', chunks[b'IHDR'])
assert depth == 8 and colour in (2, 6) and interlace == 0, f'{path}: depth {depth}, colour type {colour}, interlace {interlace}'
bpp = 3 if colour == 2 else 4
raw, stride, rows, previous = zlib.decompress(chunks[b'IDAT']), width * bpp, [], bytearray(width * bpp)
for row in range(height):
    kind, line = raw[row * (stride + 1)], bytearray(raw[row * (stride + 1) + 1:(row + 1) * (stride + 1)])
    for i in range(stride):
        a, b, c = (line[i - bpp] if i >= bpp else 0), previous[i], (previous[i - bpp] if i >= bpp else 0)
        p = a + b - c
        paeth = a if abs(p - a) <= abs(p - b) and abs(p - a) <= abs(p - c) else (b if abs(p - b) <= abs(p - c) else c)
        line[i] = (line[i] + [0, a, b, (a + b) // 2, paeth][kind]) & 255
    rows.append(line)
    previous = line
x, y = width // 2, height // 2
print(','.join(str(v) for v in rows[y][x * bpp:x * bpp + 3]))
PY
}

# What the page said last, and what Chromium said about the GPU: its
# console lines and its GPU, Dawn and Vulkan lines from Chromium's log.
console() {
  grep 'CONSOLE' "$work/$1.log" | tail -10 | sed 's#^#    #'
  grep -q 'CONSOLE' "$work/$1.log" || echo "    (no console lines in $work/$1.log)"
  echo "  $1: Chromium's GPU lines:"
  grep -iE 'gpu|dawn|vulkan|swiftshader|angle' "$work/$1.log" | grep -v CONSOLE | tail -15 | sed 's#^#    #'
}

# check <page> <#status pattern> <#error: absent | hidden | pattern>
check() {
  started=$(date +%s)
  dom=$(dump "$1") || { fail "$1: $chrome exited non-zero or passed 60 s"; console "$1"; return; }
  took=$(( $(date +%s) - started ))
  status=$(printf '%s' "$dom" | grep -o '<p id="status">[^<]*</p>' | sed 's#<[^>]*>##g')
  error_tag=$(printf '%s' "$dom" | grep -o '<p id="error"[^>]*>[^<]*</p>' || true)
  error=$(printf '%s' "$error_tag" | sed 's#<[^>]*>##g')
  echo "$1: ${took} s, #status [$status] #error [${error_tag:-absent}]"
  failed_before=$failed
  printf '%s' "$status" | grep -Eq "$2" || fail "$1: #status is [$status], wanted /$2/"
  case $3 in
    absent) [ -z "$error_tag" ] || fail "$1: #error exists, wanted none" ;;
    hidden) printf '%s' "$error_tag" | grep -q ' hidden' && [ -z "$error" ] \
              || fail "$1: #error is [$error_tag], wanted hidden and empty" ;;
    *) printf '%s' "$error_tag" | grep -q ' hidden' && fail "$1: #error is still hidden"
       printf '%s' "$error" | grep -Eq "$3" || fail "$1: #error is [$error], wanted /$3/" ;;
  esac
  [ "$failed" = "$failed_before" ] || { echo "  $1: the page's last console lines:"; console "$1"; }
}

chrome=$(browser)
version=$("$chrome" --version 2>/dev/null || echo 'version unknown')
echo "browser: $chrome ($version)"
VK_DRIVER_FILES=$(vulkan_driver "$chrome" "$version")
export VK_DRIVER_FILES
echo "vulkan driver: $VK_DRIVER_FILES (for $version)"
prepare
serve
check as-built '^adapter: ' hidden
# The canvas presented a frame. A WebGPU canvas cannot be read back once
# presented, so the pixel comes from Chromium's screenshot: the canvas
# fills the page, and the fitted gradient's centre is 128,128,128
# (docs/web.md), as the native screenshot asserts. A dead canvas is not:
# with no Vulkan driver (vkCreateInstance fails, run 39929758159) the
# device is lost at the first frame; with lavapipe alone it composites
# white (see vulkan_driver).
presented=none
if shoot as-built-shot; then
  presented=$(centre_pixel "$work/as-built-shot.png") || presented="unreadable: $presented"
fi
echo "as-built-shot: the screenshot's centre is [$presented]"
if [ "$presented" != "128,128,128" ]; then
  fail "as-built-shot: the canvas did not present the gradient (centre [$presented], wanted 128,128,128); $work/as-built-shot.png"
  echo "  as-built-shot: the page's last console lines:"; console as-built-shot
fi
if grep -q 'vkCreateInstance() failed' "$work/as-built.log"; then
  fail "Chromium has no Vulkan driver, so SwiftShader's WebGPU cannot present; fix: check that $VK_DRIVER_FILES loads (vulkaninfo --summary with VK_DRIVER_FILES set)"
fi
check no-open-button '^adapter: ' 'touch screens cannot open files'
check no-webgpu '^starting$' '^no WebGPU adapter'
check no-webgpu-no-error-element '^no WebGPU adapter' absent
check device-lost '^adapter: ' '^the GPU device was lost \(Destroyed: .*\); fix: reload the page$'
[ "$failed" = 0 ] || { echo "the page contract does not hold; reproduce: cargo xtask web && $0 target/web \$(mktemp -d)" >&2; exit 1; }
echo "ok   the page contract holds in all five cases"
