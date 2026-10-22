#!/bin/sh
# Headless check of the page contract on the built web demo: an error
# lands in #error when the page has one, in #status when it has not,
# and a status line never lands in #error.
# Usage: scripts/check-page-contract.sh target/web <empty work dir>
# after `cargo xtask web`; needs chromium and python3. Results on
# 2026-10-22 are in docs/web.md under "Page contract".
set -eu
web=$1; work=$2
[ -f "$web/index.html" ] || { echo "$web/index.html: missing; run cargo xtask web first" >&2; exit 1; }
if [ -e "$work" ] && [ -n "$(ls -A "$work")" ]; then
  echo "$work: not empty; give a new or empty directory" >&2; exit 1
fi
mkdir -p "$work"; cp "$web"/* "$work"/
cd "$work"
nogpu='<script>delete Navigator.prototype.gpu;</script>'
# 1: the page as built: a status line only
cp index.html status-only.html
# 2: no button#open, so an error after the status line
sed 's#<button id="open">Open</button>##' index.html > error-and-status.html
# 3: no WebGPU: an error, no status, with #error
sed "s#</head>#$nogpu</head>#" index.html > nogpu-with-error.html
# 4: no WebGPU, without #error
sed "s#</head>#$nogpu</head>#; s#<p id=\"error\"[^>]*></p>##" index.html > nogpu-without-error.html
python3 -m http.server 8731 --bind 127.0.0.1 >/dev/null 2>&1 &
server=$!
trap 'kill $server' EXIT
sleep 1
for page in status-only error-and-status nogpu-with-error nogpu-without-error; do
  env -u DBUS_SESSION_BUS_ADDRESS timeout 60 chromium --headless=new --no-sandbox \
    --disable-gpu-sandbox --use-angle=swiftshader --enable-unsafe-webgpu \
    --enable-features=Vulkan,WebGPU --window-size=640,400 \
    --virtual-time-budget=8000 --dump-dom "http://127.0.0.1:8731/$page.html" \
    2>/dev/null > "$page.dom"
  echo "== $page"
  grep -o '<p id="status"[^<]*</p>' "$page.dom" || echo "no #status"
  grep -o '<p id="error"[^<]*</p>' "$page.dom" || echo "no #error"
done
