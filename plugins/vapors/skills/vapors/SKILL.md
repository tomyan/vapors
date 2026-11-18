---
name: vapors
description: Build a small app or game with vapors, the Rust GUI framework whose .wisp components (Rust script, CSS, HTML template) compile to Rust and draw with wgpu on desktop and in the browser over WebGPU. Use when asked to build something with vapors or .wisp, to set up a vapors project, or to build, check, screenshot or publish a vapors web page.
---

# vapors

vapors is a Rust GUI framework. A component is a `.wisp` file with a
`<script>` in Rust, a `<style>` in CSS and an HTML template; a build
script compiles it to Rust. The page runs as wasm in the browser on
WebGPU (no WebGL fallback) and natively in a window.

Reference, read as needed:
[components](https://github.com/tomyan/vapors/blob/main/docs/components.md)
(the `.wisp` format and every refusal),
[css](https://github.com/tomyan/vapors/blob/main/docs/css.md)
(what CSS is supported),
[web](https://github.com/tomyan/vapors/blob/main/docs/web.md)
(the page contract), and the demo components in
[crates/demos/demos](https://github.com/tomyan/vapors/tree/main/crates/demos/demos).

## Set up a project

There is no vapors CLI. Make a new cargo project outside any vapors
checkout and copy the starter into it. The starter is a small game
(move a square across a road of cars with the arrow keys) built and
screenshot on 2026-11-18; start from it and change it.

```sh
mkdir mygame && cd mygame
base=https://raw.githubusercontent.com/tomyan/vapors/main/plugins/vapors/skills/vapors/starter
for f in Cargo.toml rust-toolchain.toml build.rs index.html shoot.mjs \
         src/main.rs src/game.rs src/board.wisp; do
  curl -fsSL --create-dirs -o "$f" "$base/$f"
done
cargo install trunk --locked   # or `cargo binstall trunk`; skip if `trunk --version` works
```

The files are browsable at
[starter/](https://github.com/tomyan/vapors/tree/main/plugins/vapors/skills/vapors/starter).
Rename the package in `Cargo.toml` if you like; keep the rest of it:

- Every vapors crate is a git dependency at one `rev`. Keep that pin,
  and keep all of them on the same rev. Do not switch to a branch or
  to path dependencies.
- `rust-toolchain.toml` pins the compiler vapors is built with (1.99.1)
  and the wasm target; rustup installs both on the first build.
- Generated code reaches the DOM layer through `vapors-elements`; do not
  add `vapors-dom`.

The first build compiles over 200 crates natively and about 100 for
wasm (about a minute each with 4 jobs on a 12-thread machine). trunk
downloads `wasm-bindgen` and `wasm-opt` on its first release build, so
it needs the network.

## How the starter is laid out

| File | Holds |
|---|---|
| `src/game.rs` | the rules, plain Rust with no vapors types, and their unit tests |
| `src/board.wisp` | the view: one `Signal<World>`, memos derived from it, a grid of tiles |
| `src/main.rs` | mounts the board, and wraps `ElementsApp` in an `App` that steps the game on a timer and passes key presses to it |
| `build.rs` | `vapors_compiler::build::compile_dir("src")`: every `.wisp` under `src` becomes a module (`board.wisp` is `board::Board`) |
| `index.html` | the page contract: `<canvas id="vapors">`, `#status`, `#error`, and trunk's `<link data-trunk rel="rust" />` |
| `shoot.mjs` | the headless screenshot below |

Keep game state in plain Rust and the view in `.wisp`: the rules can
then be tested with `cargo test`, and the template only reads signals.

## Check your work

Run these after each change, in this order.

1. **`wisp-check`** on each `.wisp` you changed. It reports every
   compiler refusal as `file:line:col: message`, and the message names
   the fix. Install it once:

   ```sh
   cargo install --locked --git https://github.com/tomyan/vapors \
     --rev <the rev in Cargo.toml> vapors-compiler --bin wisp-check
   wisp-check src/board.wisp   # ok: ... compiles to component `Board`
   ```

2. **`cargo test`**, natively, for the rules.
3. **`cargo build`**, natively. A rustc error in script code points at
   the generated file (`target/debug/build/<crate>-*/out/wisp/board.rs`);
   open it at that line and find the nearest
   `// from src/board.wisp:N` comment above it: that is the line to edit.
4. **`trunk build --release --public-url ./ --dist site`**. Use
   `--public-url ./` so the page loads its script from beside itself
   wherever it is served. Keep `--dist site` apart from `trunk serve`,
   which writes a debug build into `dist/`.
5. **Look at it.** Serve `site/` and screenshot it headless:

   ```sh
   (cd site && python3 -m http.server 8000 --bind 127.0.0.1 &)
   npm install playwright   # if neither playwright nor playwright-core is installed
   node shoot.mjs http://127.0.0.1:8000/ shot   # shot-1.png, then keys, shot-2.png
   ```

   Open both PNGs and look at them. The script exits 1 and prints the
   message when the page shows an error in `#error`. Change the key
   presses and waits in `shoot.mjs` to exercise your own game.

For a live view, `trunk serve` rebuilds on save and serves on
`http://127.0.0.1:8080` (a secure context, so WebGPU works there).

### When the GPU device is lost

In a container with no GPU, Chromium's WebGPU runs on SwiftShader. With
the usual flags (`--use-angle=swiftshader`, or none) Playwright's
Chromium loses the device within two frames: `#error` reads `the GPU
device was lost (Destroyed: ...)` and nothing draws
([#513](https://github.com/tomyan/vapors/issues/513)). These flags work,
and are what `shoot.mjs` uses:

```
--no-sandbox --enable-unsafe-webgpu --enable-features=Vulkan --use-angle=vulkan
VK_DRIVER_FILES=<the browser's directory>/vk_swiftshader_icd.json
```

`shoot.mjs` finds the ICD file beside the browser binary; set
`VK_DRIVER_FILES` or `CHROME` when it cannot. Unset
`DBUS_SESSION_BUS_ADDRESS` for the browser (the script does): with a
session bus but no desktop portal, every load waits 120 s.

## Writing .wisp

The rules that matter most when starting; docs/components.md has them all.

- A signal or memo needs its type written out:
  `let score: Signal<u32> = Signal::new(0);`.
- In a text hole a bare signal is read for you (`{score}`). In
  attributes, `{if}` conditions and `{for}` iterables, call `.get()`
  yourself: `{if lives.get() == 0}`.
- A `{for}` loop binding is a memo too: `{tile}` in text,
  `tile.get()` in attributes. Give a `{for}` a `key=` when rows are
  added, removed or reordered.
- Classes are literal or toggled: `class="tile" class:car={..}`.
  `class={..}` and `style={..}` are refused, because the compiler must
  see every class and declaration an element can have. Put all CSS in
  `<style>`; it is scoped to the component (`:global(body)` reaches
  outside it).
- No `{else if}` (nest an `{if}` in the `{else}`), no HTML comments
  (use `/* */` in `<style>`), and only HTML tags from the compiler's
  vocabulary; capitalised tags are child components.
- Handlers: `onclick={handler}` where `handler` is a script closure
  (`let restart = move || world.set(World::new());`) or an inline
  closure taking `&mut Event`.

## Games: what works today and the gaps

These hold at the pinned rev; each was checked against the source.

- **No frame tick or timer in `.wisp`.** `ElementsApp` draws only when
  the document changes, and its `with_per_frame` hook runs only while
  something changes every frame. The starter's `Game` wraps
  `ElementsApp` and returns `Next::WaitUntil(next_step)` so a frame
  comes at each step. For motion on every display frame, return
  `Next::Continue` instead.
- **Keys.** A `.wisp` `onkeydown` reaches only the focused element
  (one with `tabindex`) and the elements above it; with nothing focused
  keys go to `<body>`, which a component cannot listen on. The starter
  reads `vapors_platform::Event::Key { key, pressed, repeat, .. }` in
  its wrapper instead. `key` uses DOM names (`"ArrowUp"`, `" "` for
  Space); `pressed: false` is the key-up, so held keys can be tracked.
- **No positions from values.** With `style={..}` refused, nothing can
  place an element at a computed x and y. Draw the playfield as a grid
  of tiles and toggle classes, so things move a whole tile at a time
  (fine for Frogger-style games). Smooth movement would need the 3D
  scene ([scene3d](https://github.com/tomyan/vapors/blob/main/docs/scene3d.md),
  a `transform` signal per node); this skill has not tried it.
- **Time:** use `vapors_platform::Instant`. `std::time::Instant::now()`
  panics in the browser.
- **Randomness:** nothing in the dependencies provides it. A small
  hand-written generator (xorshift, seeded from the time) avoids
  setting up `getrandom` for wasm; not tried here.
- **No audio** API exists.

## Show the result

`site/` (after step 4) is the whole page: `index.html`, one `.js` and
one `.wasm`. Any static file server can serve it over https or on
localhost.

From a cloud session that has an Artifact publishing tool, publish
`site/index.html` as the page with the `.js` and `.wasm` as supporting
files under the same names (the `.wasm` with content type
`application/wasm`). Publishing that way was checked on 2026-11-18;
whether the page then runs inside the artifact viewer, which needs
WebGPU in its frame, was not. If the published page shows an error or
stays blank, say so and give the user the built `site/` directory and
your screenshots instead.
