# The web build

The platform crate runs in a browser with WebGPU. `crates/web-demo` is
the page that proves it: the image app, the gradient fixture fitted to
the canvas, pinch or scroll to zoom, drag or one finger to pan, a double
tap to toggle fit and actual size, `o` or a tap on an empty canvas to
open a file.

## What the page needs

- `<canvas id="vapors">`, sized by CSS. The platform attaches to it and
  reports its size in physical pixels; a page without it fails with a
  Windowing error naming the id.
- `<p id="status">`, optional. The adapter line is written there once the
  GPU is ready, and every error after `run()` returns (`run()` returns at
  once on the web; the browser owns the loop).
- `<button id="open">`, for touch screens. Its click reaches the app as
  `Event::OpenRequested`, and the app answers with `pick_file` and its
  file kinds: a click is user activation in every browser, a tap on the
  canvas is not always. A page without it is told so in the
  console and the status element; `o` and a tap on an empty canvas still
  try.
- CSS: `html,body{margin:0;height:100%} canvas{width:100vw;height:100vh;
  display:block;touch-action:none}`. Without `touch-action:none` the
  browser scrolls instead of sending touches.
- WebGPU: Chrome or Edge 113+, Safari 26+, Firefox 141+, over https or
  localhost. There is no WebGL fallback. `navigator.gpu` is checked
  before anything else, so a browser without WebGPU reads the `NoWebGpu`
  error and that list in the status element rather than a surface error.

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
and then trunk's rebuild lines (`starting build`, `success`, errors;
every line with `--verbose`). The dev build goes to `target/web-dev`,
apart from the static `target/web`.

It replaces whatever `tailscale serve` was serving. Ctrl-C stops trunk;
the proxy stays pointed at the dead port until `cargo xtask serve` puts
the static build back. The first build must print trunk's `server
listening at:` line within 600 s (the demo measured 30 s with its
dependencies compiled, a few minutes cold) or the command fails naming
trunk. Each missing piece is refused by name with its fix: trunk not on
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
the status element shows the bare text.

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
chromium --headless=new --no-sandbox --disable-gpu-sandbox \
  --use-angle=swiftshader --enable-unsafe-webgpu \
  --enable-features=Vulkan,WebGPU --window-size=640,400 \
  --virtual-time-budget=8000 --screenshot=web-demo.png http://127.0.0.1:8000/
```

The gradient fitted in 640x400 reads 128,128,128 at the centre and the
background 40,40,40 in the corner. `--dump-dom` in place of
`--screenshot` prints the page, with the status element's text. Chromium
155 keeps `navigator.gpu` under every disable flag, so to see the
no-WebGPU path add `<script>delete Navigator.prototype.gpu;</script>` to
the head of a copy of `index.html`.
