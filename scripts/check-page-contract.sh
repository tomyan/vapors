#!/bin/sh
# The page contract on the built web demo, in headless Chrome: an error
# lands in #error when the page has one and in #status when it has not;
# a status line never lands in #error; a lost GPU device is reported in
# #error with the fix; the demos (?demo=) draw, an unknown demo is an
# error, the page works one directory down (/vapors/), and each demo's
# screenshot, after the inputs its .pixels file lists, holds its pixel
# checks. Copies of the built page; any mismatch fails naming the case.
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

# browser and vulkan_driver: the one choice of browser and its Vulkan
# driver for every page check.
. "$(dirname "$0")/browser.sh"

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
  full_size='(Number(document.body.dataset.frameHeight) || 0) >= 400'
  page as-built "" "$adapter"
  # The screenshot's own copy, held until the canvas has drawn at the
  # screenshot's size: wgpu calls the canvas context's getCurrentTexture
  # once per frame, counted on <body> as data-frames with the canvas
  # height then as data-frame-height. Without its own page and hold the
  # screenshot was taken before any frame (vapors main, white centre,
  # after #25). Headless Chromium lays the page out 313 px tall while it
  # loads and resizes it to the 400 px of --window-size only to take the
  # screenshot, after the hold has ended (here, Chromium 155), so a hold
  # waiting for a frame at 400 px would never end. The screenshot copies
  # therefore size the canvas at 640x400 CSS px themselves, and the hold
  # waits for a frame at that size rather than for a count of frames,
  # which an app that draws only on change (F2c S2) never reaches. That
  # a resize is redrawn is the `resize` case's to check.
  counter='<script>var current = GPUCanvasContext.prototype.getCurrentTexture; GPUCanvasContext.prototype.getCurrentTexture = function () { if (document.body) { document.body.dataset.frames = String((Number(document.body.dataset.frames) || 0) + 1); document.body.dataset.frameHeight = String(this.canvas.height); } return current.apply(this, arguments); };</script>'
  frames=$(frames_at 640 400)
  page as-built-shot "$frames" "$full_size"
  # Only the image demo opens files through the page's Open button; an
  # elements demo hides it and the status line, and needs neither.
  page no-open-button "" "!error.hidden && $adapter" 's#<button id="open">Open</button>##'
  page no-open-button-elements "" "$adapter" 's#<button id="open">Open</button>##'
  page no-webgpu "$nogpu" ""
  page no-webgpu-no-error-element "$nogpu" "" 's#<p id="error"[^>]*></p>##'
  page device-lost "$lose" "!error.hidden"
  # The F2c demos (?demo=<name>), a demo name the page does not have,
  # and the page served one directory down, as the tailnet serves it
  # at /vapors/: its script and wasm load from beside it.
  page hello "" "$adapter"
  # No HTML control over an elements demo's canvas: once the adapter line
  # is written, every visible element on the canvas but #error (failures
  # stay visible) is listed on <body> as data-overlaps.
  for demo in $demos index; do
    page "$demo-overlap" "$overlaps" "(function () { if (status.textContent.indexOf('adapter: ') !== 0) return false; document.body.dataset.overlaps = vaporsOverlaps(); return true; })()"
  done
  # One screenshot of each demo per section of its .pixels file, the
  # canvas at that section's size.
  for demo in $demos; do
    sizes=$(viewports "$demo") || exit 1
    for wh in $sizes; do
      w=${wh%x*}; h=${wh#*x}; shot="$demo-shot-$wh"
      if [ -n "$($check_pixels --inputs "crates/demos/demos/$demo.pixels" "$wh")" ]; then
        page "$shot" "$(frames_at "$w" "$h")$(clicker "$demo" "$wh")" "$(after_clicks "$h")"
      else
        page "$shot" "$(frames_at "$w" "$h")" "(Number(document.body.dataset.frameHeight) || 0) >= $h"
      fi
    done
  done
  page unknown-demo "" "!error.hidden"
  # The canvas resized by CSS after its first frame, as a window resize
  # or a rotation does: the page draws only on change, so the hold ends
  # only if the resize asks for a frame and it is drawn at 300 px.
  page resize "$counter" "(function () { var f = Number(document.body.dataset.frames) || 0; if (f >= 1 && !window.vaporsResized) { var c = document.getElementById('vapors'); c.style.width = '500px'; c.style.height = '300px'; window.vaporsResized = true; } return (Number(document.body.dataset.frameHeight) || 0) === 300; })()"
  page subpath "" "$adapter"
  ln -s . "$work/vapors"
  grep -q '<p id="error"' "$work/as-built.html" || { echo "$web/index.html: no <p id=\"error\">" >&2; exit 1; }
}

# The demos whose screenshots are held to crates/demos/demos/<name>.pixels.
demos="hello counter boxes shell panel text stress toasts scroll images"
# Demos that change every frame never go idle, and headless Chromium's
# virtual time then runs hundreds of software-rendered frames past the
# 60 s deadline; `ticks=5` stops them after five frames.
animated="stress"

# query <demo>: the page's query string for <demo>.
query() {
  case " $animated " in
    *" $1 "*) echo "?demo=$1&ticks=5" ;;
    *) echo "?demo=$1" ;;
  esac
}
# The window shoot uses: 640x400, but for a demo section's screenshot.
size=640,400

# A script defining vaporsOverlaps(): the visible elements whose boxes
# meet the canvas's, other than the canvas, #error, and what the page
# copies add (style, script, the hidden hold image), as `tag#id` words,
# or `none`.
overlaps='<script>function vaporsOverlaps() { var c = document.getElementById("vapors").getBoundingClientRect(), found = []; document.querySelectorAll("body *").forEach(function (e) { if (e.id === "vapors" || e.id === "error" || e.tagName === "SCRIPT" || e.tagName === "STYLE") return; var r = e.getBoundingClientRect(); if (r.width === 0 || r.height === 0 || getComputedStyle(e).visibility === "hidden") return; if (r.left < c.right && r.right > c.left && r.top < c.bottom && r.bottom > c.top) found.push(e.tagName.toLowerCase() + (e.id ? "#" + e.id : "")); }); return found.join(" ") || "none"; }</script>'

# viewports <demo>: each section's size in the demo's .pixels as WxH, a
# line each; fails, naming the file, when check_pixels cannot read it.
viewports() {
  out=$($check_pixels --viewports "crates/demos/demos/$1.pixels") \
    || { echo "crates/demos/demos/$1.pixels: check_pixels could not read its sections" >&2; return 1; }
  printf '%s\n' "$out" | tr ' ' x
}

# frames_at <w> <h>: the screenshot copy's canvas at w x h CSS px (see
# frames in prepare), with the frame counter its hold reads.
frames_at() {
  printf '%s' "<style>canvas { width: $1px !important; height: $2px !important; }</style>$counter"
}

# clicker <demo> <WxH>: a script that makes the WxH section's inputs on
# the canvas, in the order the file lists them; check_pixels reads the
# file, so the native screenshot and this one make the same inputs. A
# click is what a mouse sends (move, press, release, pointer id 1; a move
# has no button and lists itself as its coalesced event, which winit
# reads). A wheel moves the pointer there, then sends a WheelEvent in
# pixels (deltaMode 0), as Chromium sends for a notch on Linux.
clicker() {
  calls=$($check_pixels --inputs "crates/demos/demos/$1.pixels" "$2" | while read -r kind x y dx dy; do
      case $kind in
        click) printf 'vaporsClick(%s, %s); ' "$x" "$y" ;;
        wheel) printf 'vaporsWheel(%s, %s, %s, %s); ' "$x" "$y" "$dx" "$dy" ;;
        *) echo "crates/demos/demos/$1.pixels: an input check_pixels printed is neither click nor wheel: $kind" >&2; exit 1 ;;
      esac
    done) \
    || { echo "crates/demos/demos/$1.pixels: check_pixels could not read its inputs" >&2; exit 1; }
  printf '%s' "<script>function vaporsMove(c, x, y) { var move = { clientX: x, clientY: y, bubbles: true, pointerId: 1, pointerType: 'mouse', isPrimary: true, button: -1, buttons: 0 }; move.coalescedEvents = [new PointerEvent('pointermove', move)]; c.dispatchEvent(new PointerEvent('pointermove', move)); } function vaporsWheel(x, y, dx, dy) { var c = document.getElementById('vapors'); vaporsMove(c, x, y); c.dispatchEvent(new WheelEvent('wheel', { clientX: x, clientY: y, deltaX: dx, deltaY: dy, deltaMode: 0, bubbles: true, cancelable: true })); } function vaporsClick(x, y) { var c = document.getElementById('vapors'); var o = { clientX: x, clientY: y, bubbles: true, pointerId: 1, pointerType: 'mouse', isPrimary: true, button: 0, buttons: 1 }; var move = { clientX: x, clientY: y, bubbles: true, pointerId: 1, pointerType: 'mouse', isPrimary: true, button: -1, buttons: 0 }; move.coalescedEvents = [new PointerEvent('pointermove', move)]; c.dispatchEvent(new PointerEvent('pointermove', move)); c.dispatchEvent(new PointerEvent('pointerdown', o)); o.buttons = 0; c.dispatchEvent(new PointerEvent('pointerup', o)); } function vaporsClicks() { $calls}</script>"
}

# after_clicks <h>: the hold condition. The clicks wait for a frame
# at the screenshot's height (see frames in prepare: a click made on the
# first, 313 px tall layout lands on another element); the hold then
# ends once a frame after the clicks has begun, the one they asked for.
# The page draws only on change, so a click that asks for no frame never
# ends the hold, and `held` reports the server's timeout.
after_clicks() {
  echo "(function () { var f = Number(document.body.dataset.frames) || 0; if ((Number(document.body.dataset.frameHeight) || 0) < $1) return false; if (window.vaporsClickedAt === undefined) { vaporsClicks(); window.vaporsClickedAt = f; } return f > window.vaporsClickedAt; })()"
}

# held <page>: fails when the server let <page>'s hold go at its 30 s
# deadline instead of the page's own condition: what the page was
# waiting for never happened, and a dump or screenshot taken anyway
# shows the page before it.
held() {
  if [ -f "$work/timeouts" ] && grep -qx "$1" "$work/timeouts"; then
    fail "$1: the page's hold timed out at 30 s; its condition never held (a frame or a status that never came)"
    return 1
  fi
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
        if not flag(self.path).wait(30):
            with open('timeouts', 'a') as out:
                out.write(self.path.split('/', 2)[2] + '\\n')
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
    --virtual-time-budget=8000 --dump-dom "$(url "$@")" 2>"$work/$1.log"
}

# url <page> [query] [directory]: where a copy is served; a query such
# as ?demo=hello picks the demo, and vapors/ loads it one directory
# down, as the tailnet serves the page at /vapors/.
url() {
  echo "http://127.0.0.1:$port/${3:-}$1.html${2:-}"
}

# The page as Chromium draws it, as $work/<page>.png, after the page's
# own hold; its stderr in $work/<page>.log. The window is $size, `W,H`.
shoot() {
  env -u DBUS_SESSION_BUS_ADDRESS timeout 60 "$chrome" --headless=new --no-sandbox \
    --disable-gpu-sandbox --use-angle=swiftshader --enable-unsafe-webgpu \
    --enable-features=Vulkan,WebGPU --window-size="$size" --enable-logging=stderr --v=0 \
    --virtual-time-budget=8000 --screenshot="$work/$1.png" "$(url "$@")" 2>"$work/$1.log"
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

# check <page> <#status pattern> <#error: absent | hidden | pattern> [query] [directory]
check() {
  started=$(date +%s)
  dom=$(dump "$1" "${4:-}" "${5:-}") || { fail "$1: $chrome exited non-zero or passed 60 s"; console "$1"; return; }
  took=$(( $(date +%s) - started ))
  status_tag=$(printf '%s' "$dom" | grep -o '<p id="status"[^>]*>[^<]*</p>' || true)
  status=$(printf '%s' "$status_tag" | sed 's#<[^>]*>##g')
  error_tag=$(printf '%s' "$dom" | grep -o '<p id="error"[^>]*>[^<]*</p>' || true)
  error=$(printf '%s' "$error_tag" | sed 's#<[^>]*>##g')
  echo "$1: ${took} s, #status [$status] #error [${error_tag:-absent}]"
  held "$1" || true
  failed_before=$failed
  printf '%s' "$status" | grep -Eq "$2" || fail "$1: #status is [$status], wanted /$2/"
  case $3 in
    absent) [ -z "$error_tag" ] || fail "$1: #error exists, wanted none"
            # Errors go to #status then, so it must be shown.
            printf '%s' "$status_tag" | grep -q ' hidden' && fail "$1: #status holds the error but is hidden" ;;
    hidden) printf '%s' "$error_tag" | grep -q ' hidden' && [ -z "$error" ] \
              || fail "$1: #error is [$error_tag], wanted hidden and empty" ;;
    *) printf '%s' "$error_tag" | grep -q ' hidden' && fail "$1: #error is still hidden"
       printf '%s' "$error" | grep -Eq "$3" || fail "$1: #error is [$error], wanted /$3/" ;;
  esac
  [ "$failed" = "$failed_before" ] || { echo "  $1: the page's last console lines:"; console "$1"; }
}

chrome=$(browser)
version=$("$chrome" --version 2>/dev/null) && [ -n "$version" ] || {
  echo "$chrome --version failed or printed nothing; fix: check the browser starts (run it with --version)" >&2
  exit 1
}
echo "browser: $chrome ($version)"
VK_DRIVER_FILES=$(vulkan_driver "$chrome" "$version")
export VK_DRIVER_FILES
echo "vulkan driver: $VK_DRIVER_FILES (for $version)"
# The demos' pixel checker, built before the server starts so its
# compile does not count against any page's deadline.
check_pixels=${CHECK_PIXELS:-}
if [ -z "$check_pixels" ]; then
  cargo build -q -p vapors-pixels --bin check_pixels || { echo "cannot build check_pixels; reproduce: cargo build -p vapors-pixels --bin check_pixels" >&2; exit 1; }
  check_pixels="cargo run -q -p vapors-pixels --bin check_pixels --"
fi
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
if shoot as-built-shot '?demo=image' && held as-built-shot; then
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
check no-open-button '^adapter: ' 'touch screens cannot open files' '?demo=image'
check no-open-button-elements '^adapter: ' hidden '?demo=hello'
check no-webgpu '^starting$' '^no WebGPU adapter'
check no-webgpu-no-error-element '^no WebGPU adapter' absent
check device-lost '^adapter: ' '^the GPU device was lost \(Destroyed: .*\); fix: reload the page$'
check hello '^adapter: ' hidden '?demo=hello'
check unknown-demo '^starting$' '^no demo is called `nope`' '?demo=nope'
check subpath '^adapter: ' hidden '' vapors/
check resize '^adapter: ' hidden '?demo=hello'
# overlap <demo>: no visible HTML control over the elements demo's canvas.
overlap() {
  dom=$(dump "$1-overlap" "$(query "$1")") || { fail "$1-overlap: $chrome exited non-zero or passed 60 s"; console "$1-overlap"; return; }
  held "$1-overlap" || return
  found=$(printf '%s' "$dom" | grep -o 'data-overlaps="[^"]*"' | sed 's#data-overlaps="\(.*\)"#\1#')
  echo "$1-overlap: HTML over the canvas: [${found:-not measured}]"
  [ "$found" = none ] || fail "$1-overlap: [${found:-not measured}] lies over the canvas of an elements demo; hide it (vapors_platform::hide_page_controls) or move it off the canvas"
}
for demo in $demos index; do overlap "$demo"; done
# Each demo as Chromium shows the canvas after its clicks, held to the
# same crates/demos/demos/<name>.pixels as the native screenshot, by the
# same code, one screenshot per section at that section's window size;
# check_pixels picks the section by the screenshot's size.
shots=""
for demo in $demos; do
  sizes=$(viewports "$demo") || exit 1
  for wh in $sizes; do
    shot="$demo-shot-$wh"; shots="$shots $shot"; size="${wh%x*},${wh#*x}"
    if shoot "$shot" "$(query "$demo")" && held "$shot"; then
      $check_pixels "crates/demos/demos/$demo.pixels" "$work/$shot.png" || { fail "$shot: the canvas fails demos/$demo.pixels; $work/$shot.png"; console "$shot"; }
    else
      fail "$shot: $chrome exited non-zero or passed 60 s"; console "$shot"
    fi
  done
done
[ "$failed" = 0 ] || { echo "the page contract does not hold; reproduce: cargo xtask web && $0 target/web \$(mktemp -d)" >&2; exit 1; }
echo "ok   the page contract holds in all ten cases, no HTML over the canvas of: $demos index; the gradient screenshot and the screenshots:$shots"
