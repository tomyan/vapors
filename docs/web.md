# The web build

The platform crate runs in a browser with WebGPU. `crates/web-demo` is
the page that proves it: the image app, the gradient fixture fitted to
the canvas, pinch or scroll to zoom, drag or one finger to pan, a double
tap to toggle fit and actual size, `o` or a tap on an empty canvas to
open a file.

## Page contract

The elements the platform looks for, by id (`vapors_platform::page`):

- `<canvas id="vapors">`, sized by CSS. The platform attaches to it and
  reports its size in physical pixels; a page without it fails with a
  Windowing error naming the id.
- `<p id="status">`, optional. `report_status` writes there: the adapter
  line once the GPU is ready, and what was opened. Each line replaces
  the last.
- `<p id="error" hidden>`, optional. `report_error` writes there and
  removes the `hidden` attribute, so a page can tell an error from a
  status line by the element rather than by console text. Errors come
  after `run()` returns too (`run()` returns at once on the web; the
  browser owns the loop). A page without `#error` gets its errors in
  `#status`. `report_status` never writes to `#error`. Both also go to
  the console, errors with `console.error`.
- `<button id="open">`, for touch screens, placed top right so it does
  not cover the label strip the image app draws top left. Its click reaches the app as
  `Event::OpenRequested`, and the app answers with `pick_file` and its
  file kinds: a click is user activation in every browser, a tap on the
  canvas is not always. A page without it is told so as an error; `o`
  and a tap on an empty canvas still try.
- CSS: `html,body{margin:0;height:100%} canvas{width:100vw;height:100vh;
  display:block;touch-action:none}`. Without `touch-action:none` the
  browser scrolls instead of sending touches.
- WebGPU: Chrome or Edge 113+, Safari 26+, Firefox 141+, over https or
  localhost. There is no WebGL fallback. `navigator.gpu` is checked
  before anything else, so a browser without WebGPU reads the `NoWebGpu`
  error and that list (in `#error`, or `#status`) rather than a surface
  error.

`scripts/check-page-contract.sh target/web <new or empty dir>`, after
`cargo xtask web`, loads four copies of the built page in headless
Chrome and checks each one's `#status` and `#error`; a mismatch fails
naming the case. CI runs it in the `web` job with the runner's
`google-chrome`; the browser is `$CHROME`, else `google-chrome`, else
`chromium`. The cases: the page as built shows the adapter line in
`#status` and `#error` hidden and empty; without the Open button, the
adapter line in `#status` and the missing-button error in `#error`;
without WebGPU, the `NoWebGpu` error in `#error` and `#status` still
`starting`; without WebGPU and without `#error`, the error in
`#status`. Verified on Chromium 155 here on 2026-10-22.

## Build

```
cargo xtask web          # trunk build --release into target/web; prints the size
cargo xtask wasm         # the CI lint: clippy for wasm32-unknown-unknown
```

`cargo xtask ci` runs `wasm` between `clippy` and `unit`. The package is
`web.package` in `.config/factory.toml`. Trunk downloads `wasm-opt` on
the first release build.

## Serve on the tailnet

```
cargo xtask serve        # sudo tailscale serve --bg --https=443 target/web; prints the URL
```

The URL is `https://<Self.DNSName>/` from `tailscale status --json`. Open
it on a phone on the same tailnet. `sudo tailscale serve reset` stops it.
sudo is run with `-n`: when it would ask for a password the command fails
and prints the line to run by hand.

## Develop: reload on save, on the phone

```
cargo xtask dev          # trunk serve (debug) on 127.0.0.1:8080 behind tailscale serve; Ctrl-C stops
```

`trunk serve` builds the web package in debug (seconds, not the
release minute), watches the whole workspace, rebuilds on every save
and serves the result on `127.0.0.1:<port>` with its reload script
injected into the page; `[dev] port` in `.config/factory.toml` sets
the port, default 8080. `sudo -n tailscale serve --bg --https=443
http://127.0.0.1:<port>` puts that server on the tailnet over https, so
the page on the phone reloads on save; the command prints the URL once
and then trunk's rebuild lines (`starting build`, `success`, errors, and
after an error every line until the next build so rustc's locations and
`help:` show; every line with `--verbose`). The dev build goes to `target/web-dev`,
apart from the static `target/web`.

It replaces whatever `tailscale serve` was serving. Ctrl-C in the
terminal stops trunk (they share the terminal); the proxy stays pointed
at the dead port until `cargo xtask serve` puts the static build back.
There is no signal handler by decision: a SIGTERM or SIGKILL to xtask
orphans trunk and its cargo build, and `pkill trunk` is the recovery.
Every failure after trunk has started kills it and, on Unix, the cargo
build in its process group. The first build must print trunk's `server
listening at:` line or accept a TCP connection on the port within 600 s
(the demo measured 30 s with its dependencies compiled, a few minutes
cold) or the command fails naming trunk with its last 40 lines. Each missing piece is refused by name with its fix: trunk not on
PATH (`cargo xtask setup`), the port in use, tailscale not on PATH, sudo
wanting a password (the line to run by hand is printed).

`http://127.0.0.1` is a secure context, so WebGPU initialises on the
dev server as it does over https.

## The native file picker

`Platform::pick_file(&[FileKind])` takes what to offer: each kind is a
description and its extensions, mapped to the native dialog's filters and
to the web input's `accept` (`image/*` plus every extension, so phones
offer their photo library). An empty list is refused.
`report_status(&str)` writes a non-error line, stdout natively and the
status element on the web; the image app says `opened <name> <w>x<h>`.
Lines on stdout, stderr and the console start with `WindowOptions.app_name`
(or the title's first word when it is empty), so touche's say `touche:`;
the `#status` and `#error` elements show the bare text.

## Device limits

The window device asks for the default limits with `max_buffer_size`,
`max_texture_dimension_2d` and `max_storage_buffer_binding_size` raised
to what the adapter reports, so a large photo's texture and readback fit;
`Frame::limits()` says what was granted, and the adapter status line
states the texture and buffer ceilings. The GPU test harness opens its
device the same way. An adapter below the default for any of the three
is refused by name.

On Linux the picker asks the XDG desktop portal over D-Bus
(rfd's `xdg-portal` backend; nothing to install at build time, a portal
service at run time). What the portal says when it fails is reported as
`file dialog: <cause>` on stderr; the only wait that is not the user's is
the portal's reply to `OpenFile`, bounded by libdbus at 25 s. macOS and
Windows use their native dialogs.

## Screenshot headlessly

From inside `target/web`, serve it and shoot it with Chromium's
software WebGPU (verified on Chromium 155, Debian 13, 2026-10-22).
SwiftShader rasterises on the CPU, so the capture waits 8 s of virtual
time for the first frame:

```
(cd target/web && python3 -m http.server 8000 --bind 127.0.0.1 &)
env -u DBUS_SESSION_BUS_ADDRESS \
  chromium --headless=new --no-sandbox --disable-gpu-sandbox \
  --use-angle=swiftshader --enable-unsafe-webgpu \
  --enable-features=Vulkan,WebGPU --window-size=640,400 \
  --virtual-time-budget=8000 --screenshot=web-demo.png http://127.0.0.1:8000/
```

`env -u DBUS_SESSION_BUS_ADDRESS` matters on a box with
xdg-desktop-portal installed but unable to start (no display):
Chromium's network service asks the portal over the session bus before
its first request, and D-Bus activation waits 120 s, so every HTTP
load stalls past any deadline. With no session bus address it skips
the portal; curl and file:// are never affected.

The gradient fitted in 640x400 reads 128,128,128 at the centre and the
background 40,40,40 in the corner. The top 44 px are the label strip
(23,27,31) with `gradient.png  <zoom>%` on it in Geist; `--dump-dom`
cannot see canvas text, so to check the label read the strip's pixels
for values that are not the strip colour. `--dump-dom` in place of
`--screenshot` prints the page, with the status element's text. Chromium
155 keeps `navigator.gpu` under every disable flag, so to see the
no-WebGPU path add `<script>delete Navigator.prototype.gpu;</script>` to
the head of a copy of `index.html`.
