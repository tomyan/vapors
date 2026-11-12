# Threading: where more than one thread would help vapors

After the performance finish line (`docs/plans/perf-finish-line.md`):
no slice here starts before every property there holds, measured,
unless Tom moves it. Design only; nothing here is built, and no cargo
command was run for it. Read from the code at 90d1c8a (origin/main,
2026-11-12) and from the design batch on `docs/design-batch` (97fd1f6:
`docs/perf/timed-2026-11-12.md`, `splice-layout.md`, `text-measure.md`,
`style-allocations.md`). Line numbers are from those revisions. A
number marked "measured" names its run; every other number is an
estimate for the slice's measurement to replace. Browser and toolchain
facts are cited to a source found on 2026-11-12 or marked "not
verified".

This plan keeps the threading rule of
`docs/design/02-scheduler-frame-threading.md` ("Threading") and
`docs/design/ARCHITECTURE.md:38-42`: the UI thread owns the graph, the
tree, the scene and the GPU queue's submission order; other threads
take owned data in and give owned data out, and wake the loop through
`Waker`. Where this plan says more than that page, it says so; where it
would differ, section 6 asks.

Binding rules it is written under: runtime performance first (an
edit's cost flat in page size; idle does nothing); no API whose effect
the compiler's analysis cannot see
(`docs/decisions/2026-11-11-analysable-sources.md`); a small shared
runtime; every failure loud; no `unsafe` in vapors' own code
(`Cargo.toml:79`, `unsafe_code = "forbid"`).

## Summary

- An edit is 53 to 303 µs of update (measured, section 1.1). A thread
  hand-off is tens of µs (estimate, section 1.3). Nothing on an edit's
  path is worth a thread, and the finish line's own slices are taking
  the O(rows) terms out of insert, scroll and resize single-threaded.
  Threading is for the first frame of a large page, for work that is
  not the frame (decoding an image), and for the GPU-side frame of a
  game. Section 1 says which, with the numbers.
- One mechanism covers what is worth doing natively: a fixed pool of
  parked threads that takes owned jobs and returns owned results over
  a channel, woken by `Waker`, with a deadline on every wait (section
  2.1; design 02's S4 `offload`, not built). Shaping at mount runs on
  it as a batch before layout (section 2.3), image decoding as a
  future (2.6). Neither changes an `Rc` in the tree.
- A render thread (2.5) needs `Painted` to be `Send` (`Rc` to `Arc` in
  `slab.rs`) and buys nothing until a frame is CPU-bound on a
  continuous workload; property 9's run decides. Parallel style,
  layout and paint (2.2, 2.4) are not recommended: their per-row cost
  is falling under the finish-line slices faster than a thread change
  could, and each needs `Arc` on a hot path.
- The web (section 3): a worker pool with copies works today on the
  pinned stable toolchain; shared memory (wasm threads) needs a nightly
  toolchain with `-Z build-std` as of 2026-11-12 and cross-origin
  isolation headers the served page does not set yet. Recommended:
  copies only until `build-std` is stable.
- Seven slices (section 5), five decisions (section 6).

## 0. Where things stand

| Part | As built | Where |
|---|---|---|
| Reactive graph | One thread-local runtime, `Rc<RefCell<..>>` nodes, no locks; "the automation API posts closures to the UI thread" | `crates/signals/src/runtime.rs:27-29`, `node.rs:34-37`, `lib.rs:9-14` |
| Document | Shared as `Rc<RefCell<Document>>`, which generated code takes; event handlers take `&Rc<RefCell<Document>>` | `crates/elements/src/document.rs:14-15`, `keys.rs:36`, `events_mouse.rs:23` |
| Update | `style`, then `layout`, then `paint`, in sequence on one `&mut self`; each a `tracing` span | `crates/elements/src/update.rs:59-86` |
| Style | Parents first: `restyle_subtree` walks `elements(root)` in order; `restyle_one` compares `Rc::ptr_eq` and keeps `Rc` styles | `restyle_pass.rs:118-125`, `147-166` |
| Layout | Taffy over `BoxTree`, one `&mut` tree holding `&mut FontSystem` and `&mut TextCache`; text is shaped and broken inside `measure_text` when Taffy asks | `layout/tree.rs:19-33`, `layout/measure.rs:12-29` |
| Text cache | `Rc<Layout>` entries, swept by update | `layout/text_cache.rs:36-45` |
| Painted | Slabs of `Rc<Layout>`, `Rc<RgbaImage>`, `Rc<Icon>`; hits in a `OnceCell` | `slab.rs:69-74`, `painted.rs:34` |
| Frame | Input, tasks, tick, update, accessibility, draw, record, in sequence; `tasks` and `tick` are no-op defaults (design 02's S2 and S3 not built) | `crates/app/src/frame.rs:146-163`, `117-119`, `elements_app_frame.rs:28-55` |
| Draw | Record into one `FrameEncoder` (`RefCell` inside, `&self` shared), then `submit` on the same thread | `elements_app_frame.rs:86-92`, `crates/renderer/src/frame_encoder.rs:45-55` |
| Threads today | `Waker` is `Send` natively; `RedrawFlag` is atomic; the native file picker runs on its own thread; GPU setup blocks on the UI thread natively and is a `spawn_local` future on the web | `crates/platform/src/waker.rs:1-6`, `redraw.rs:8-10`, `picker.rs:22`, `tasks.rs:17-28` |
| `offload` | Designed (`docs/design/01-reactivity-async.md` section 4, design 02 S4); not built: no `offload` in `crates/` | grep at 90d1c8a |
| Image decode | `decode_png` is synchronous on the UI thread | `crates/canvas/src/png.rs:15`, `image_app.rs:79` |
| Perf counter | Counts the calling thread's user-space instructions only | `crates/perf/src/instructions.rs:1-6` |
| Toolchain | Stable 1.99.1 pinned; trunk 0.22.0, wasm-bindgen-cli 0.2.129 | `rust-toolchain.toml`, `.config/tools.toml` |
| Served page | static-web-server with compression and cache headers; no COOP or COEP | `xtask/src/serve_static.rs:1-6` (grep for `Cross-Origin`: none) |
| touche | Its own design puts the engine on a thread natively and in workers on the web, single-threaded on the web "until isolation headers exist" | `~/projects/touche/DESIGN.md:155-158`, `189`, `243` |

The dev machine has 12 hardware threads (`nproc`, 2026-11-12).

## 1. What the numbers say about parallelism

### 1.1 Measured: what a frame costs

`docs/perf/timed-2026-11-12.md` (main at 9b405f9, three runs, medians,
µs):

| workload | update | style | layout | paint | draw | GPU time |
|---|---|---|---|---|---|---|
| 3000 rows, Idle | 7 | 1 | 3 | 0 | 488 | 270 |
| 3000 rows, Text | 60 | 6 | 41 | 8 | 507 | 270 |
| 3000 rows, Class | 303 | 241 | 32 | 25 | 725 | 270 |
| 10000 rows, Idle | 19 | 3 | 8 | 0 | 1223 | 270 |
| 3000 rows, Insert | 10,612 | 307 | 3,774 | 6,422 | 1,617 | 272 |
| 3000 rows, Scroll | 21,580 | 3 | 1,982 | 19,298 | 666 | 273 |
| racer, racing | 569 | 26 | 445 | 80 | 1,853 | 4,763 |

Inside draw, `encode`, `write` and `submit` are about 120 to 130 µs
each at idle (the lead's reading of the same run, not in the published
table). Idle draws no frames at all outside the harness (property 6;
`Next::Wait`), so the idle row is the cost of a frame that something
asked for.

The first frame of a page (`docs/perf/mount-shared-2026-11-12.md`,
after fixes, instructions per extra row at 10,000 rows): style
344,326, layout 244,805, paint 29,231, mount 29,184; 6.2 G instructions
for the whole first frame at 10,000 rows, 0.3 to 2.4 s of wall time
across the three sizes (`mount-2026-11-12.md`, "Load", taken at load
36, not kept). Of layout's 245,000 per row, the text is 110,000 to
125,000 after text-measure's option A (`text-measure.md`, section 3:
a shaping about 48,000 instructions, a break about 16,000; 20,001
shapings at 10,000 rows), the rest Taffy's passes over 13 boxes.

### 1.2 Where the finish line leaves each cost

| Cost | Today | After the finish line's slices | Threading's room |
|---|---|---|---|
| An edit (Text, Class, FlexText) | 53 to 303 µs, flat in rows (property 1 holds) | style-allocations S1 takes a Class edit's style from 845k to about 644k instructions | none: under the hand-off cost of 1.3 |
| Insert | 10.6 ms at 3000 rows | splice-layout PRs 1 to 5: about an edit plus log n | none, once the lane lands; before it, the O(rows) terms are per-row work in five owners' code (`splice-layout.md` 1.3), parallelisable in principle and removed instead |
| Scroll, Resize | 21.6 ms at 3000 rows | IU4 4-ii and IU3 3c-3e: O(visible) | none |
| Draw | 0.5 ms at 3000 rows idle, 1.2 ms at 10,000 | IU4: kept GPU side, O(visible) | a render thread overlaps draw with the next update only when frames are continuous (2.5) |
| First frame, 10,000 rows | 6.2 G instructions | U1, U2, B0 to B3b (`build-time-styles.md`), TM1, TM2 (`text-measure.md`): style from 344k per row toward the share key's cost; text from 3 breaks to 2 and 2 width-only answers | shaping and breaking stay per row and are independent per run: the one large parallelisable term (2.3) |
| Racer, racing | 2.5 ms CPU, 4.8 ms GPU | one-scene (property 9): the track drawn as scene nodes | GPU-bound today; a render thread helps only when CPU recording exceeds the GPU's time (2.5) |
| Opening an image in touche | `decode_png` on the UI thread; a 24 MP PNG is hundreds of ms (estimate, not measured in vapors) | nothing on the finish line | the whole decode leaves the UI thread (2.6) |

### 1.3 The hand-off cost

Giving work to another thread and taking the result back costs, on
Linux: a channel send, a futex wake of a parked thread (tens of µs
when the thread is asleep; a few µs when it is spinning, which a pool
that "does nothing idle" must not do), the work, a send back and a
wake or a poll. Estimate: 20 to 100 µs round trip to a parked thread;
measured in slice TH0 before any other slice is accepted. Against an
edit's 53 to 303 µs that is a loss or a wash; against a 10,000-row
first frame's text (about 2.4 G instructions, 1.2 G of them shaping)
it is nothing. The pool is used for batches of at least a few
milliseconds and never inside an edit's update.

The reverse cost is what matters for the finish line: a change that
makes the single-threaded path slower by even 2% on `--scaling` fails
the judge. Every option below lists what it adds to that path.

## 2. Options per area

### 2.1 The pool: one mechanism, native

A fixed pool of `N - 1` threads (`std::thread::available_parallelism`,
capped at 8; the cap is a number for TH0's measurement to replace),
started on the first job and parked on a channel until one arrives
(`std::sync::mpsc`; no spinning, so an idle pool costs no CPU and no
wake-ups). A job is owned data in, owned data out: a `Box<dyn FnOnce()
-> Result<R, E> + Send>` with a `Cancel` token (design 01 section 3,
an `Arc<AtomicBool>` the drop sets). Results come back on a per-job
channel; the pool wakes the loop through `Waker` so a resolved job is
seen in the Tasks phase (design 02 S2's executor; until it exists, the
app's `event` handles a `Message` the platform delivers).

Two ways to wait, both with a deadline:

- **A future** (design 01's `offload`): the UI thread never waits; the
  job's deadline is the caller's (`Instant` from a measurement of that
  job type: TH0 records them), and passing it resolves the future as
  `Unreachable(Reason::Deadline)` naming the job, so the three-state
  rule holds and the stall is reported where the caller is.
- **A batch join** (section 2.3): the UI thread submits K jobs and
  `recv_timeout`s each result with one deadline for the batch, set
  from the single-threaded time of the same batch times a factor
  (TH3 measures the factor; the number and where it came from are in
  the code). A deadline passed is a panic naming the batch, the jobs
  outstanding and the fix, not a silent fall-back to shaping on the
  UI thread: a worker that hangs on pure shaping is a bug the same
  as a hang on the UI thread would be, and must be seen.

Why not rayon: the jobs here are coarse (a whole decode; a batch of
runs split into N chunks), and the rule asks for a small shared
runtime. A pool with a channel and a `Cancel` is about 150 lines
with its tests; rayon's work stealing pays off on fine-grained
recursive splits none of 2.2 to 2.6 need. If TH3's measurement shows
uneven chunks leaving threads idle, chunking by cost (a run's text
length) is the next step before a stealing scheduler. `std::thread::
scope` is not used: it joins without a deadline, and its borrowed
jobs would let a worker hold a reference into the tree, which the
threading rule forbids.

Ownership changes: none in the tree. The pool lives in a new crate
(`crates/pool`, `vapors-pool`), depended on by `vapors-platform`
(which owns `Waker`) and `vapors-elements` (for 2.3). Nothing in it is
`cfg(wasm)`; the web build gets a different implementation of the same
trait in the platform crate (section 3), per ARCHITECTURE.md's rule
that `cfg(target_family = "wasm")` stays in platform modules.

Cost to the single-threaded path: zero until the first job; the pool's
threads exist after that and are parked. Binary: about 150 lines.

### 2.2 Parallel mount and style

**Mount.** Generated code calls `Dom` on `Rc<RefCell<Document>>` and
reads signals on the thread-local runtime; the mount is 29,000
instructions per row, 2.7% of the first frame (`mount-2026-11-12.md`).
Not parallelisable without a second graph runtime, and not worth it.

**Style.** `restyle_subtree` is parents first because an element's
cascade reads its parent's computed style (`restyle_one`,
`style_for`), and the share table keys on the parent style's address
(`restyle_share.rs`). A fork-join per level (style the parent, then
its children in parallel) would need: `Rc<Counted>` styles to be
`Arc` (`style/styles.rs:47-52`), the share table to take concurrent
inserts, per-thread output merged into `Kept`'s `NodeMap`s, and the
`UpdateCounts` to be atomics. The `Arc` change puts an atomic
increment on every style clone on the edit path too (a Class edit
restyles four elements: a handful of clones, under 100 instructions;
within noise, but a cost on the path the judge watches, for a gain
only at mount). After U2 and B3b the per-element cost at mount is
the share-key lookup plus `Used` per distinct context, which the
build-time-styles plan estimates at a few thousand instructions per
element from today's 86,000. Recommendation: no parallel style.
Revisit only if M1's published first-frame time is over a target Tom
sets and the profile then shows style still the largest term.

### 2.3 Text shaping at mount and on new rows: a batch before layout

The one term that is both large and independent per unit. At mount
every row's three runs are shaped and broken when Taffy first asks
`measure_text`; after T1 (#253, #254) a run is shaped once per
content and scale (`Shaped`, `crates/text/src/shaped.rs:17-21` on
`perf/t1-text-cache`: a `parley::Layout<Color>`, a `String`, an
`f32`; owned, no `Rc`) and broken per width. Shaping needs a
`FontSystem` (`FontContext` and `LayoutContext`); breaking needs the
`Shaped` and a width.

The design: after the build phase of layout has made the rebuilt
boxes and before Taffy runs, `Document` collects the `InlineRun`s
(`layout/inline.rs:29-34`: an owned `String`, a `TextStyle`, spans)
of rebuilt boxes whose shape key is not in the text cache, hands them
to the pool in N chunks with the scale, and takes back `Shaped` per
key, inserting each into the cache on the UI thread (the cache keeps
`Rc`, made there). Taffy then runs as today and every shaping is a
cache hit. Breaking stays on the UI thread inside `measure_text`:
after TM1 a Mount row breaks twice, at the width Taffy found, which
the batch cannot know in advance; a second batch could break at the
max-content width, which fits the common case (text-measure 1.1: on
Mount W equals max-content), and is a follow-up once the shaping
batch is measured.

What a worker needs:

- Its own `FontSystem`. fontique's `Collection` has a `shared` option
  whose clones share one store (`fontique-0.12.0/src/collection/mod.rs:
  37-46`), and `SourceCache` the same (`source_cache.rs:24-32`); parley
  says `FontContext` and `LayoutContext` are "resources which should be
  shared globally (or at coarse-grained boundaries)". No `Rc` or
  `RefCell` in parley 0.12 or fontique 0.12 outside the fontconfig
  backend, which carries `unsafe impl Send for SystemFonts`
  (`backend/fontconfig.rs:601`). So a `FontSystem` per thread, cloned
  from the UI thread's with `shared: true` at pool start, is the
  shape; that `FontSystem: Send` holds is asserted by a compile-time
  test in TH3 (a function with a `Send` bound), not assumed.
- Owned input: the run's text and styles are cloned into the job. A
  10,000-row page clones 30,000 short strings once, under 1 M
  instructions against the 1.2 G shaped (estimate).
- Owned output: `Shaped` is `Send` if `parley::Layout<Color>` is;
  the same compile-time assertion covers it.

Expected gain (estimate; TH3 measures): at 10,000 rows shaping is
about 1.2 G of the frame's 6.2 G instructions; with 7 workers and a
hand-off of under 1 ms, wall time falls by about 1.0 G instructions'
worth, roughly 0.3 s of a first frame of about 2 s on this machine.
At 1,000 rows, about 30 ms of 0.3 s. The instruction total rises by
the clones and the channel traffic, under 1%. The same batch serves
an insert of rows with new text (the `Insert` workload reuses its
text, `text-measure.md` 1.3, so a new `InsertNew` workload measures
it, section 4) and touche's layer list when it mounts.

Cost to the single-threaded path: the collection pass over rebuilt
boxes (a walk the build already does; a vector of keys per update,
empty on an edit whose text is cached), under 1,000 instructions on
an edit (estimate; `--scaling` judges it). With the pool size 1 or the
batch under its minimum size (a threshold in instructions from TH0's
hand-off measurement, say 20 runs), the batch is skipped and nothing
changes, so an edit never pays a hand-off.

### 2.4 Layout and paint off or across threads

**Layout off the UI thread** means the whole update moves, because
Taffy's tree borrows the boxes, the fonts and the text cache
mutably, and placement, stackings and paint follow it in one `&mut
self` (`update.rs:66-80`). That is the render-in-a-worker shape of
design 02 (the whole frame on another thread, input forwarded by
message), not a parallel layout. Natively its gain is a UI thread
free during an update it would otherwise be busy in, at the cost of
one hop of input latency per event, and the update is 60 to 300 µs:
no.

**Parallel layout across subtrees.** M10c's isolation (`layout/
isolated.rs`) lays a changed owner out alone and replays what its
parent asked; independent owners could be laid out on different
threads if each had its own `FontSystem` and text cache and the
results were merged. Taffy's cache and `written` are per tree. The
owners that are large (the rows container) are the ones the
splice-layout lane makes O(log n). No.

**Parallel paint.** Painting a box into its part (`paint_partial.rs`)
reads its rect, style, text layouts and images and writes a part;
parts are independent and the order is a separate structure
(`KeptOrder`). Paint is 29,000 instructions per row at mount (2.8%)
and 8 to 40 µs on an edit. Would need `Rc<Layout>` and `Rc<RgbaImage>`
in the slabs to be `Arc`. No.

### 2.5 Render encoding on a separate thread, native

Natively wgpu's `Device` and `Queue` are `Send + Sync` (wgpu 30;
`wgpu-types-30/src/send_sync.rs` makes `WasmNotSend` plain `Send` off
wasm). A render thread could take an owned snapshot of what draw
reads, record the encoder and either submit itself or hand the
`CommandBuffer` back for the UI thread to submit (the threading rule
keeps submission order on the UI thread; design 02 "Deliberately left
out" allows encoding elsewhere). What draw reads: `Painted` (the
slabs' `Rc<Layout>`, `Rc<RgbaImage>`, `Rc<Icon>`; `KeptBatches`;
`Frames` after IU4), the painter's pipelines, atlas and retained
buffers (`ScenePainter`, `crates/app/src/painter.rs:14-23`), and the
surfaces (`painter_surfaces.rs`: each `SurfaceRenderer` is app code
sharing its scene as `Rc<RefCell<..>>`, `crates/scene3d-gpu/src/lib.rs:
5-7`).

The ownership change: the three slabs to `Arc` (`slab.rs:69-74`), the
hits' `OnceCell` left on the UI side (the render thread does not hit
test), the painter owned by the render thread with the UI thread
sending it a `DrawList` per frame (batches, the frame table, the
atlas's new glyphs as owned coverage), and surfaces rendered on the UI
thread before the hand-off (their renderers are app code and hold
`Rc`). Cost on the single-threaded path: `Arc` increments where the
slabs clone, a few per update; the `DrawList` copy per frame, O(visible
batches), about 52 batches on the rows page. Both within noise, both
judged by `--scaling`.

The gain: draw's CPU time (488 µs at 3000 rows idle today; IU4 brings
it down) overlaps the next frame's update. On a single edit the time
to pixels is unchanged: update and draw still run in sequence for
that frame. On a continuous workload (the racer, a scroll held down)
the UI thread's per-frame time falls from update + draw to update.
Today the racer is GPU-bound (4.8 ms GPU against 2.5 ms CPU, 16.7 ms
per frame), so there is nothing to gain until property 9's one-scene
run shows a frame whose CPU side is the longer. Recommendation: design
recorded here, built only on that measurement (decision 4).

The web is different: a `GPUDevice` cannot be shared between threads
(design 02, gpuweb issue 354), so the only form is the whole frame in
a worker (design 02 S6), which this plan does not repeat.

### 2.6 Asset decoding: `offload`

`decode_png` (`crates/canvas/src/png.rs:15`) runs on the UI thread
when the image app opens a file (`image_app.rs:79`); touche's opener
will decode JPEG, PNG and 16-bit TIFF with the `image` crate and RAW
later (`DESIGN.md:180`, `1238`). A 24 MP decode is the one case today
where the UI thread is visibly blocked for longer than a frame.
Design 01 section 4 and design 02 S4 specify `offload`: a named free
function taking owned input and a `Cancel`, run on the pool natively
and on a worker pool on the web, returning a future resolved in the
Tasks phase. The decode is the first user; export encoding the second.
Nothing in the tree changes; the result is an `RgbaImage` handed to a
signal write.

What is new over design 01 here: the deadline on the future (2.1),
and that the decode's input is the file's bytes (owned `Vec<u8>`,
transferred to a worker on the web without a copy as an
`ArrayBuffer`) and its output the pixels (a copy back on the web
without shared memory: 96 MB for 24 MP RGBA8, about 30 ms at memcpy
speed, estimate; one of section 3's reasons shared memory is worth a
later slice for touche, not for vapors' own pages).

### 2.7 A simulation thread for games

The racer's model steps in microseconds (`crates/racer/src/model.rs:
58-82`) on the UI thread; particles run on the GPU
(`docs/plans/particles.md`, decided). A game whose simulation is
CPU-bound would run it on its own thread at a fixed timestep, writing
each step's state into a double buffer the UI thread reads at the
Tasks phase as one signal write (owned snapshot: positions, headings;
`Send` plain data), with input forwarded as owned events the other way.
That is the threading rule as it stands; the app owns the thread and
the snapshot type, vapors provides the pool, the `Waker` and the Tasks
phase. No vapors change is needed beyond 2.1; recorded so nothing
below rules it out (Tom, 2026-11-12, in `particles.md`: 3D eventually
suitable for games). Not sliced until a game workload exists.

## 3. The web

### 3.1 What the page runs on today

One thread. `vapors_platform::run` returns at once and the browser
owns the loop (`docs/web.md`); GPU setup is a `spawn_local` future
(`crates/platform/src/tasks.rs:24-28`); the a11y mirror writes the DOM
(R6), IME and clipboard are DOM shims. The wasm is built by trunk on
the stable toolchain for `wasm32-unknown-unknown` without atomics
(`xtask/src/web.rs`, `rust-toolchain.toml`).

### 3.2 Workers with copies

A `Worker` runs its own instance of the same wasm module (its own
linear memory). Input crosses by structured clone or as a transferred
`ArrayBuffer` (no copy); output the same. A running wasm function
cannot see a `Cancel` token (no shared cell), so cancel is
`worker.terminate()` and a replacement from the pool (design 01
section 3). The function must be named, not a closure, so the worker's
module can find it: design 01's `#[vapors::offload]` registry.

Costs: each worker instantiates the module (the page's wasm is 1.7 MB
of brotli for touche, `docs/design/text-stack-size.md`; instantiation
per worker is not measured here), so the pool starts lazily and is
small (`navigator.hardwareConcurrency` capped at 4, a number for TH2
to measure); each worker's memory is a full instance; results are
copied back. For a decode (bytes in, pixels out) that is one copy of
the pixels. For a shaping batch (2.3) the output per run is small and
the font data would be loaded per worker (the bundled Geist fonts are
in the module as `include_bytes`, so no fetch; the fontique collection
is built per worker, not measured).

This path needs nothing the pinned toolchain does not have. It is the
web half of TH1 and TH2.

### 3.3 Shared memory: wasm threads

Needs, as of 2026-11-12:

- **Cross-origin isolation.** `SharedArrayBuffer` exists only on a
  page served with `Cross-Origin-Opener-Policy: same-origin` and
  `Cross-Origin-Embedder-Policy: require-corp` (or `credentialless`,
  whose Safari support is not verified; see sources). `crossOriginIsolated`
  on `window` reports it. `require-corp` blocks cross-origin resources
  loaded without CORS or a `Cross-Origin-Resource-Policy` header;
  touche's API calls are CORS requests and are unaffected, and both
  pages load no third-party resources. GitHub Pages cannot set headers
  (touche `DESIGN.md:156`); static-web-server can (TH4 adds them to
  `serve_unit.rs`'s config).
- **A toolchain that builds `std` with atomics.** Every source found
  (the wasm-bindgen-rayon docs, a 2026 guide to threaded wasm) still
  builds with a nightly toolchain and `-Z build-std=panic_abort,std`
  plus `-C target-feature=+atomics,+bulk-memory`; `build-std` is an
  accepted 2026 Rust project goal with RFCs in progress, not
  stabilised. That is against `rust-toolchain.toml`'s pinned stable
  1.99.1; a nightly for the web build alone is a factory decision
  (decision 3). The 2026 guide also reports the linker no longer
  infers shared memory from `+atomics` and needs `--shared-memory` and
  `--import-memory` explicitly: not verified against trunk 0.22.0.
- **Two builds.** A module built with shared memory cannot instantiate
  on a page without `SharedArrayBuffer`, so a page that must also run
  on a non-isolated host (or a browser without the headers honoured)
  ships two wasms and picks by `crossOriginIsolated` at load; design
  02 says the choice is reported in the status line.
- **wgpu stays on the main thread.** With atomics on, wgpu's types are
  not `Send` on wasm (the `fragile-send-sync-non-atomic-wasm` feature
  is defined as having no effect under atomics; `wgpu-30.0.1/Cargo.toml:
  65` still lists it). The frame's GPU work does not move; only pool
  jobs do.
- **wasm-bindgen-rayon** (1.3.0, found 2026-11-12) gives rayon over
  web workers on shared memory with a `nightly` feature; its docs pin
  a 2024 nightly. If TH4 is built, an own pool over `std::thread::spawn`
  (which works under atomics with wasm-bindgen's threads transform) is
  the smaller runtime and the same code as native; wasm-bindgen-rayon
  is the fallback if the transform's set-up proves longer than the
  crate's.

What it buys over 3.2: no copies (a 24 MP decode's pixels read in
place, a `Cancel` the worker polls), and workers that share the font
collection and the module. For vapors' own pages nothing today needs
it; for touche's engine it is the design (`DESIGN.md:243`). Recommend:
copies only (3.2) until `build-std` is stable, then TH4 (decision 3).

### 3.4 WebGPU in workers and OffscreenCanvas

For design 02 S6 (the whole frame in a worker), not for this plan's
pool: `navigator.gpu` is exposed in workers (MDN, `WorkerNavigator.gpu`,
secure context); Chrome 124 added it to service and shared workers.
WebGPU on an `OffscreenCanvas` transferred to a worker: Chrome yes
(design 02 cites 113+); Firefox and Safari not verified (the searches
found Firefox's WebGL-in-worker path and a Mozilla note "WebGPU
OffscreenCanvas not yet supported" of unknown date, and a WebKit bug
on WebGL in workers). Desktop WebGPU itself: Chrome and Edge 113+,
Firefox 141+ (Windows, then Apple-silicon macOS; Linux behind a flag
per one source), Safari 26+ (`docs/web.md`, web.dev). S6 stays gated
on a measurement, as design 02 says, and its page contract would
check the `OffscreenCanvas` WebGPU context before offering it.

## 4. Measurement plan

Instruction counts are the acceptance numbers for the single-threaded
path (they do not move with load); wall time is the gain of a thread,
and only a quiet-window run can read it (the finish-line method:
load under 1.5, three runs, medians).

- **Two instruction counts.** `InstructionCounter` counts the calling
  thread (`instructions.rs:1-6`). perf-event2 0.7.4's `Builder::inherit`
  (`builder.rs:488`) makes a counter follow threads the counted thread
  creates after it opens. TH0 adds `--threads`: a second counter opened
  with `inherit` before the pool starts, so the report shows `ui`
  (the UI thread's instructions: the latency side) and `all` (every
  thread's: the cost side) per phase. A parallel slice is accepted
  when `all` rises by under 3% and the phase's wall time falls by the
  predicted share; the `ui` count is what `--scaling` judges and must
  not rise on any edit workload.
- **The hand-off.** TH0 adds a `Handoff` micro-workload: an empty job
  round trip to a parked pool thread, 1000 times, reported in µs and
  instructions; the number sets 2.3's minimum batch size and the
  pool's deadline factors, and is cited where they are set.
- **Workloads.** `Mount` at 1,000, 3,000 and 10,000 rows (first-frame
  wall time, `layout` phase, `texts_shaped`); `InsertNew` (a row with
  a text no other row has, so the insert shapes); `Decode` (a 24 MP
  PNG offloaded: the UI thread's longest frame, the decode's total
  time, both targets); the racer racing (for 2.5: CPU frame against
  GPU time). Every existing `--scaling` workload runs unchanged with
  the pool on and with `--threads 1`, and the two must agree within
  noise on every phase.
- **The web.** `scripts/web-idle-frames.sh`'s harness runs the
  `Decode` case in headless Chromium and reads the main thread's
  longest task from the page-contract script (a `PerformanceObserver`
  on `longtask`); TH2 pins it under one frame.
- **Published.** `docs/perf/threads-<date>.md` per slice: the two
  counts, wall times, the pool size, load.

## 5. Slices

Each under about 400 production lines; each check proven by breaking
what it guards, said in the commit; the count pins in `sparse_updates.
rs` unchanged unless the PR says which and why.

| # | Slice | Content | Accepted when | Depends on |
|---|---|---|---|---|
| TH0 | Measurement | `--threads` in `vapors-perf`: the `all` counter with `inherit`, `ui` beside it, both in the report and `--scaling`'s judge reading `ui`; the `Handoff` workload; `InsertNew`; a compile-time test that `vapors_signals` types are `!Send` (design 02's oracle, not present at 90d1c8a: grep) | `docs/perf/threads-<date>.md` with the hand-off in µs and instructions; every `--scaling` workload unchanged; the `!Send` test proven by breaking (an `unsafe impl Send` in a test module fails it) | after the finish line |
| TH1 | The pool, native | `crates/pool`: parked threads, `mpsc` jobs, `Cancel`, per-job deadline, `Waker` wake; `offload` natively as design 01 section 4 (the attribute can come later; a plain function `offload(job, deadline) -> Offloaded<T>` first); the Tasks phase drains resolved jobs into their signal writes (the minimal part of design 02 S2); `decode_png` offloaded in the image app | `Decode` natively: the UI thread's longest frame under 2 ms with a 24 MP PNG (today: the decode's whole time); idle after the decode: 0 frames in 3 s (`idle_fps`); a deadline passed resolves `Unreachable` naming the job, proven by breaking (a job that sleeps past it); a dropped future stops the worker within one step (the `Cancel` test of design 02's oracle) | TH0 |
| TH2 | Workers with copies, web | The platform's worker pool: a `Worker` per slot running the page's own module, the registry of named functions, structured clone in, transfer where the type allows, `terminate()` on cancel; the same `offload` call; the pool size and start reported once in the status line | `Decode` on the built page in headless Chromium: longest main-thread task under 16 ms; the same result bytes as native, pinned; `check-page-contract.sh` has the case; the page without workers (a host that blocks them) refuses the offload naming the fix, not silently decoding on the main thread | TH1 |
| TH3 | Shaping batch | 2.3: `FontSystem` per pool thread (fontique `shared`), the collection pass of rebuilt boxes' uncached runs, N chunks, `Shaped` back into the cache before Taffy; the batch skipped under the minimum size and at `--threads 1`; compile-time `Send` assertions for `FontSystem` and `Shaped`; the batch deadline with its factor and source in the code | Mount at 10,000 rows: `texts_shaped` unchanged, `layout` wall time down by the shaping share times (1 − 1/threads) within 20% of the prediction, `all` up under 3%; `--scaling` every workload unchanged at `--threads 1` and with the pool on; `InsertNew` layout wall time down the same way; a deadline passed panics naming the batch, proven by breaking | TH0, TH1, T1 (#253, #254) and TM1 merged |
| TH4 | Shared memory, web | COOP and COEP in `serve_unit.rs`'s config; `crossOriginIsolated` read at load and reported; a second wasm built with atomics on a nightly pinned in `tools.toml` (its own `[toolchain]` entry) and `-Z build-std`, picked at load; `std::thread` workers over shared memory for the same pool trait; `Cancel` as an atomic; `Decode` without the copy back | Both pages (vapors demos, touche) load isolated on the tailnet and not isolated from a file copy, each reporting which; `Decode` on the isolated page copies nothing back (bytes counted); a non-isolated load runs TH2's path; the `web` CI job builds both | TH2; decision 3 |
| TH5 | Render thread, native | 2.5: `Arc` slabs, `DrawList`, the painter on its thread, surfaces rendered before the hand-off, submission on the UI thread; `--threads 1` keeps today's path | The racer racing (or whatever property 9's run shows CPU-bound): UI-thread frame time down by draw's share, `all` up under 3%; `--scaling` unchanged; pixels unchanged (`check_pixels` on `hello`, the screenshot tests) | property 9 measured; decision 4 |
| TH6 | Breaking batch | 2.3's follow-up: break at max-content width in the batch when the run is reusable, so Mount's remaining break per label leaves the UI thread | Mount: `text_layouts_built` on the UI thread falls by one per label; the same counts as TH3 | TH3 |

Order: TH0, TH1, TH2 are one line (the pool and its two targets);
TH3 after T1 and TM1 land, since it shapes what they keep; TH4 and
TH5 wait on their decisions; TH6 last.

## 6. Decisions for Tom

1. **Build any of this before the finish line?** (a) No: every
   property first; TH0 at the earliest after property 9's run. (b)
   TH1 now, for touche's image open. **Recommend (a)**: the brief
   says after the line, and the finish-line slices are taking the
   large single-threaded terms out at a rate a pool would not match.
2. **First slice after TH0: the pool with the decode (TH1, TH2) or
   the shaping batch (TH3)?** (a) TH1 and TH2: the decode is a visible
   stall in touche today and changes nothing in the tree. (b) TH3
   first: the only term on the finish line's own workloads. **Recommend
   (a)**; TH3 needs TH1's pool anyway.
3. **A nightly toolchain for the web build (TH4).** (a) Not until
   `build-std` is stable; the web gets workers with copies. (b) A
   pinned nightly for the web job now, two wasms. **Recommend (a)**:
   the cost is a second toolchain in every gate and CI run and a
   build whose flags changed under users in the past year (section
   3.3's source), for a gain only touche's engine needs.
4. **The render thread (TH5).** (a) Only on a measurement from
   property 9's run showing a CPU-bound frame. (b) Build it with the
   one-scene work so the racer's frame is measured both ways.
   **Recommend (a)**: the ownership change touches the slabs every
   paint reads, for no gain while frames are GPU-bound.
5. **Headers on the served pages now?** (a) Not until TH4: nothing
   uses them, and `require-corp` is a rule every later resource must
   meet. (b) Set them now so the page contract can report
   `crossOriginIsolated` early. **Recommend (a)**.

Implementation choices recorded here, not for Tom: an own pool over
rayon (2.1); `std::thread::scope` not used (2.1); `FontSystem` per
thread rather than a lock (2.3); the pool in its own crate; the web
pool in the platform crate (ARCHITECTURE.md's `cfg` rule).

## 7. Risks

1. **The hand-off is larger than estimated.** TH0 measures it first;
   if a round trip to a parked thread is over 200 µs on this machine,
   TH3's minimum batch rises and its gain at 1,000 rows may be nothing,
   which the acceptance row then says.
2. **`FontSystem` is not `Send`.** Read from the sources (2.3), not
   compiled. If fontique's fontconfig backend or parley's contexts hold
   something `!Send` on some platform, TH3 builds the per-thread
   `FontSystem` from the bundle alone (`FontSystem::bundle_only`) on
   the workers, which is what the web gets anyway, and the UI thread's
   keeps system fonts; a run needing a system fallback is shaped on
   the UI thread, counted.
3. **Shaping results differ by thread.** They must not: the same
   fonts, scale and text give the same `Shaped`. TH3's debug build
   shapes a sample of the batch again on the UI thread and panics
   naming the run on a difference (the kept-paint check's pattern,
   `crates/elements/src/kept_paint.rs`; #249's `painter_kept_check.rs`
   is not on main at 90d1c8a).
4. **`Send` bounds leaking into the tree.** A worker that could hold a
   reference into the tree is what the rule forbids; the pool's job
   type is `'static + Send` and takes owned data only, and the
   `!Send` test on the graph (TH0) catches a graph type reaching a
   job.
5. **Per-thread counting undercounts.** `inherit` follows threads
   created after the counter opens; a pool started before it is not
   counted. TH0 opens the `all` counter before the pool starts and
   asserts the pool's threads were created after, failing the run
   otherwise.
6. **The web pool's memory.** Each worker is a module instance; four
   workers of touche's page is four linear memories. TH2 reports the
   instance size and start time and caps the pool by them.
7. **Toolchain drift for shared memory.** Section 3.3's source
   reports threaded-wasm builds breaking under rustc and wasm-ld
   changes within a year. TH4, if built, pins the nightly in
   `tools.toml` and the gate refuses another, as it does every tool.
8. **A worker that never answers.** Every wait has a deadline (2.1);
   the UI thread's `recv_timeout` is the only wait, and passing it is
   a loud failure naming the job. A pool thread that panics is
   reported through the job's channel closing, which resolves the job
   `Unreachable(Reason::WorkerPanicked)` with the panic message.
9. **Browser support for the frame in a worker** (3.4) is unverified
   for Firefox and Safari; it gates design 02 S6, not this plan.

Sources read on 2026-11-12 for section 3 (none of them a browser
vendor's compatibility table; each claim above that rests on one is
marked):
[rust-project-goals build-std (2026)](https://rust-lang.github.io/rust-project-goals/2026/build-std.html),
[wasm-bindgen-rayon docs](https://docs.rs/wasm-bindgen-rayon),
[Building threaded wasm in 2026](https://garrett-build.pages.dev/posts/silent-regressions-wasm-threads),
[MDN WorkerNavigator.gpu](https://developer.mozilla.org/en-US/docs/Web/API/WorkerNavigator/gpu),
[Chrome: WebGPU in workers (124)](https://developer.chrome.com/blog/new-in-webgpu-124),
[web.dev: WebGPU in major browsers](https://web.dev/blog/webgpu-supported-major-browsers),
[Bevy PR 12205 on wgpu's wasm Send feature](https://github.com/bevyengine/bevy/pull/12205),
[StackBlitz on COEP credentialless](https://blog.stackblitz.com/posts/cross-browser-with-coop-coep/),
[web.dev OffscreenCanvas](https://web.dev/articles/offscreen-canvas).
