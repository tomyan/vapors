---
name: vapors
description: Build a small app or game for the browser with vapors, the Rust GUI framework whose .wisp components (Rust script, CSS, HTML template) compile to Rust and draw with wgpu on WebGPU. Use when asked to build something with vapors or .wisp, to set up a vapors project, or to build, check, screenshot or publish a vapors web page.
---

# vapors

vapors is a Rust GUI framework. A component is a `.wisp` file with a
`<script>` in Rust, a `<style>` in CSS and an HTML template; a build
script compiles it to Rust. The page runs as wasm in the browser on
WebGPU (there is no WebGL fallback). vapors also runs natively in a
window; this skill covers the browser only.

No model has seen `.wisp` in training. Read these before writing any:

- [reference/wisp.md](reference/wisp.md): the format, every feature
  with an example that compiles.
- [reference/supported.md](reference/supported.md): the elements, CSS
  properties, selectors and events that exist, generated from the
  source. Anything else is refused.
- [reference/runtime.md](reference/runtime.md): the Rust side: starting
  the page, signals, the frame loop, and the 3D scene games draw with.

## Set up, and start the first build at once

The first build takes about a minute for a debug build and a minute and
a half for a release build on 4 cores, so start it before designing
anything. Run this block as it is, as your first action. It assumes
x86_64 Linux, `curl` and `tar`.

```sh
command -v rustup || curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
mkdir -p mygame ~/.local/bin && cd mygame
base=https://raw.githubusercontent.com/tomyan/vapors/main/plugins/vapors/skills/vapors/starter
for f in Cargo.toml rust-toolchain.toml build.rs index.html shoot.mjs \
         src/main.rs src/game.rs src/view.rs src/page.wisp; do
  curl -fsSL --create-dirs -o "$f" "$base/$f"
done
cargo generate-lockfile   # also installs the pinned toolchain and wasm target
wb=$(sed -n '/^name = "wasm-bindgen"$/{n;s/^version = "\(.*\)"$/\1/p;}' Cargo.lock)
gh=https://github.com
curl -fsSL $gh/trunk-rs/trunk/releases/download/v0.22.0/trunk-x86_64-unknown-linux-gnu.tar.gz \
  | tar -xz -C ~/.local/bin trunk
curl -fsSL $gh/wasm-bindgen/wasm-bindgen/releases/download/$wb/wasm-bindgen-$wb-x86_64-unknown-linux-musl.tar.gz \
  | tar -xz -C ~/.local/bin --strip-components=1 wasm-bindgen-$wb-x86_64-unknown-linux-musl/wasm-bindgen
curl -fsSL $gh/WebAssembly/binaryen/releases/download/version_133/binaryen-version_133-x86_64-linux.tar.gz \
  | tar -xz -C ~/.local/bin --strip-components=2 binaryen-version_133/bin/wasm-opt
nohup trunk build --offline > build.log 2>&1 &   # the first debug build, in the background
```

`--offline` stops trunk from downloading its own tools: it fails if the
prebuilt ones above are missing, rather than fetching others. Never
`cargo install` these tools; it compiles them for minutes.

The starter is a small game: arrow keys move a frog across a road of
cars, drawn as a 3D scene seen from above, with the score in HTML over
it. Change it into what you are asked for. Keep the dependencies in
`Cargo.toml` as they are:

- Every vapors crate is a git dependency at one `rev`. Keep that pin
  and keep them all equal; never switch to a branch or a path.
- Their default features are off; that keeps the release wasm at 5.5 MB
  rather than 8.5 MB.
- `rand` works in the browser through `getrandom`'s `wasm_js` feature,
  already set. Use `vapors_platform::Instant`, never
  `std::time::Instant`, which panics in the browser.

| File | Holds |
|---|---|
| `src/game.rs` | the rules, plain Rust with no vapors types, with unit tests |
| `src/page.wisp` | the view and the game loop: the scene, the keys, the score |
| `src/view.rs` | where things are drawn: the camera, the lens, a square |
| `src/main.rs` | builds the scene renderer and runs the page |
| `index.html` | the page contract (runtime.md) |
| `shoot.mjs` | the headless screenshot below |

Keep game state in plain Rust and the view in `.wisp`, so `cargo test`
checks the rules natively.

## Check your work

After each change, in this order:

1. **`wisp-check`** on each `.wisp` you changed: it prints every
   refusal as `file:line:col: message` naming the fix, in about a
   second. Install it once, in the background since it compiles for a
   minute:
   `cargo install --locked --git https://github.com/tomyan/vapors --rev <the rev in Cargo.toml> vapors-compiler --bin wisp-check`.
2. **`cargo test`** for the rules.
3. **`trunk build --offline`** (debug, into `dist/`). A rustc error in
   script code points at the generated file
   (`target/wasm32-unknown-unknown/debug/build/<crate>-*/out/wisp/page.rs`);
   the nearest `// from src/page.wisp:N` comment above that line names
   the line to edit.
4. **Look at it.** Serve `dist/` and screenshot it headless:

   ```sh
   (cd dist && python3 -m http.server 8000 --bind 127.0.0.1 &)
   npm install playwright   # unless playwright or playwright-core is installed
   node shoot.mjs http://127.0.0.1:8000/ shot   # shot-1.png, then keys, shot-2.png
   ```

   Open both PNGs and look at them. The script exits 1 and prints the
   message when the page shows an error in `#error`; some refusals
   (layout, CSS values) appear only there, when the page runs. Change
   the keys and waits in `shoot.mjs` to exercise your own game.

`trunk serve --offline` rebuilds on save and serves on
`http://127.0.0.1:8080`, a secure context, so WebGPU works there.

### When the GPU device is lost

In a container with no GPU, Chromium's WebGPU runs on SwiftShader. With
the usual flags Playwright's Chromium loses the device within two
frames: `#error` reads `the GPU device was lost (Destroyed: ...)`
([#513](https://github.com/tomyan/vapors/issues/513)). These work, and
`shoot.mjs` uses them:

```
--no-sandbox --enable-unsafe-webgpu --enable-features=Vulkan --use-angle=vulkan
VK_DRIVER_FILES=<the browser's directory>/vk_swiftshader_icd.json
```

`shoot.mjs` finds the ICD file beside the browser binary; set
`VK_DRIVER_FILES` or `CHROME` when it cannot. It also unsets
`DBUS_SESSION_BUS_ADDRESS`: with a session bus but no desktop portal,
every page load waits 120 s.

## Rough edges

- A `<button>` or `<input>` must be a flex or grid item, and a text
  input needs a width; otherwise the page shows an error when it runs.
  `br`, lists, tables and `label` are not drawn yet (supported.md).
- An element's position cannot follow a signal (`style={..}` is
  refused), so anything that moves is a mesh in the 3D scene
  (runtime.md).
- Pending, not yet in the pinned rev: `frame_time()` (the per-frame
  clock the starter's cars move by) and `<vapors:window onkeydown>`
  (the starter's keys). Until the pin moves to a rev that has them,
  the starter does not build.
- Props of the 3D tags (`<Mesh>`, `<Scene>`, ...) are checked by rustc,
  not `wisp-check`; a wrong one is a rustc error in the generated file.

## Publish

Build for release, then publish what it produces:

```sh
trunk build --offline --release --public-url ./ --dist site
```

`site/` holds `index.html`, one `.js` and one `.wasm`. `--public-url ./`
lets the page load them from beside itself wherever it is served. If
the session has an Artifact tool, publish `site/index.html` as the page
with the `.js` and the `.wasm` as supporting files under the same names
(the `.wasm` with content type `application/wasm`). Otherwise give the
user the `site/` directory: any static server over https serves it.
