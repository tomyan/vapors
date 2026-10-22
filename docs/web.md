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
- CSS: `html,body{margin:0;height:100%} canvas{width:100vw;height:100vh;
  display:block;touch-action:none}`. Without `touch-action:none` the
  browser scrolls instead of sending touches.
- WebGPU: Chrome or Edge 113+, Safari 26+, Firefox 141+, over https or
  localhost. There is no WebGL fallback; a browser without WebGPU reads
  the `NoWebGpu` error and that list in the status element.

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
`--screenshot` prints the page, with the status element's text.
