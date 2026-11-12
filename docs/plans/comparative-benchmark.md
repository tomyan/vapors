# Comparative benchmark

After the performance finish line (`docs/plans/perf-finish-line.md`,
"After the line", decided by Tom 2026-11-11): the js-framework-benchmark
operations and 7GUIs, run natively against GPUI, Blitz, iced, Slint and
egui with the same harness method as `vapors-perf`, published with every
result, including where vapors loses. The website follows it.

Design only; nothing here is built. Framework facts are cited to their
source or marked "not verified"; versions are those found on 2026-11-12
and are pinned again at slice time. Sections 1 to 6 follow the
recommendation of each decision in section 8; where another option
would change a section, the decision says which.

## 0. What is being compared, and what is not

The question the benchmark answers: for the same application, built the
way each framework's documentation says to build it, how long from a
user's action to the frame that shows its result, on the CPU, and how
does that change with the size of the page. It is a benchmark of the
frameworks' update paths (state change, layout, paint, draw submission),
the part of vapors the finish line worked on.

It is not:

- A rendering-quality comparison. Text shaping, antialiasing and colour
  are out of scope; the harness checks only that the operation happened.
- A startup or binary-size comparison; both are reported as context
  (section 4) and not scored.
- A browser comparison. js-framework-benchmark measures web frameworks
  inside Chrome; this benchmark borrows its operations and its row
  layout, not its numbers, and no result here is comparable with a
  number on the js-framework-benchmark results page (section 1.1 says
  why).

## 1. The operations

### 1.1 js-framework-benchmark, and how it measures

Upstream (krausest/js-framework-benchmark; its `webdriver-ts/src/
benchmarksCommon.ts` and `benchmarksPlaywright.ts`, read 2026-11-12)
runs a web page with a table of rows and six buttons (run, run lots,
add, update, clear, swap rows) inside Chrome, and times nine
operations, each from one button click:

| # | name | timed | warm-up before timing | CPU throttle |
|---|---|---|---|---|
| 01 | run1k | create 1,000 rows | 5 cycles of run, check, clear | none |
| 02 | replace1k | run again on 1,000 rows | 5 | none |
| 03 | update10th1k_x16 | append " !!!" to every 10th row's label | 3 | 4x |
| 04 | select1k | click row 2's label; it gets class `danger` | 10 extra runs | 4x |
| 05 | swap1k | swap rows 2 and 999 (1-based) | | 4x |
| 06 | remove-one-1k | remove row 4 | | 2x |
| 07 | create10k | create 10,000 rows | | none |
| 08 | create1k-after1k_x2 | append 1,000 rows to 1,000 | | none |
| 09 | clear1k_x8 | clear 1,000 rows | | 4x |

(The README's table says 08 appends to 10,000 and 09 clears 10,000;
the code uses 1,000.) Rows have an id from a counter that starts at 1
and never resets, and a label of a random adjective, colour and noun
(25, 10 and 13 words). A row is a `<tr>` with the id, an `<a>` with the
label, an `<a>` with the remove glyph and an empty cell; clicking the
label selects the row.

**What an operation's time is**: from Chrome's trace, the start is the
click's `EventDispatch` event and the end is the first `Commit` on the
main process after the last of the frame callback, timer, layout and
function-call events that follow it; the duration is the commit's end
minus the click's timestamp (the project wiki, "How the duration is
measured"). So the number is the browser's own main-thread time to the
compositor commit, measured by the browser's tracer, after Chrome's
CPU throttling where the table says so. 15 iterations per operation,
none dropped; the result holds min, max, mean, median, geometric mean
and standard deviation (which one the table shows is not verified;
likely the median). Memory (ready, after run, after run and clear),
startup (Lighthouse) and sizes are reported separately.

**The score** ("Computation of the weighted geometric mean", the
wiki): per operation, a framework's factor is its duration over the
fastest framework's; the score is the geometric mean of the factors,
weighted per operation by one over the 90th-percentile factor across
frameworks (01 0.64, 02 0.56, 03 0.56, 04 0.19, 05 0.13, 06 0.53, 07
0.56, 08 0.55, 09 0.42), so an operation on which most frameworks are
close counts for less.

**Keyed and non-keyed**: a keyed implementation keeps one DOM node per
data row across updates (a swap moves nodes; a remove deletes the
node), and the results table separates the two. Every application here
is keyed in that sense: a row's widget identity follows the row.

**Why none of this transfers**: the native number here is a frame loop
driven by a harness, not a trace of Chrome's main thread; there is no
throttling; the end condition is the framework's idle after its frame
and the GPU's completion (2.1), not a compositor commit; the machine
differs. The operations and the row layout are borrowed so that a
reader who knows upstream knows what each operation does; no number
here is to be set beside an upstream number, and the results page says
so at the top (5). The weights are not borrowed either: they come from
the spread across web frameworks on each operation, and the operation
set here is a subset (D5).

### 1.2 The operations run here

The six Tom named (`perf-finish-line.md`, "After the line"), upstream's
01, 03, 04, 05, 06 and 07. 02, 08 and 09 are not in the set; each is
one driver operation and one check more, and can be added by a slice
after B6 if wanted. For each: the pre-state the harness restores before
every sample, untimed; the input; the post-state the check reads (2.4).

| operation | pre-state | input | post-state checked |
|---|---|---|---|
| create 1,000 | empty page | click "run" | 1,000 rows (bands to the viewport's edge; the model's count read as a second, weaker check) |
| update every 10th | 1,000 rows, labels fresh | click "update" | rows 1, 11, 21 changed in the label cell; 2 and 12 not |
| select | 1,000 rows, none selected | click row 2's label | row 2's background the selected colour; rows 1 and 3 not |
| swap | 1,000 rows | click "swap rows" | rows 2 and 999 exchanged (D4) |
| remove | 1,000 rows | click row 4's remove glyph | 999 rows; row 4's cell holds what row 5's did |
| create 10,000 | empty page | click "run lots" | 10,000 rows (bands; the model's count) |

The pre-state restore is the application's own clear and run (so it
costs what the framework costs), untimed, followed by the pre-state
check. Labels come from upstream's word lists with a seeded generator
so that every framework shows the same labels in the same order, and
the id counter never resets, as upstream's does not. Rows are 1-based
in this document as in upstream's descriptions; the spec (6) gives
them as 0-based indices.

### 1.3 7GUIs

Seven tasks (eugenkiss.github.io/7guis/tasks), specified as behaviour
and judged qualitatively upstream; 7GUIs gives no timing method, so
the operations timed here are this document's (7, 7G0 to 7G5), and
the results page says so.

- **Counter**: a label with a number and a button that increments it.
  Timed: one click to the frame.
- **Temperature Converter**: two text fields, Celsius and Fahrenheit,
  bidirectional (C = (F - 32) x 5/9); a non-numeric entry leaves the
  other field unchanged. Timed: one keystroke in Celsius to the frame
  showing Fahrenheit.
- **Flight Booker**: a combo box (one-way, return), two date fields, a
  Book button; the return field is enabled only for a return flight;
  a date that does not parse colours its field red and disables the
  button; a return before the departure disables the button. Timed: a
  keystroke that makes the first date invalid, to the frame; the combo
  change to the frame.
- **Timer**: a gauge of elapsed time, a label, a slider for the
  duration, a reset button; the gauge runs while elapsed is under the
  duration. Timed: CPU per frame while running; frames drawn after the
  duration is set below the elapsed time (an idle measurement, as the
  finish line's property 6).
- **CRUD**: a prefix filter on the surname, a list of names, name and
  surname fields, Create, Update and Delete, the last two needing a
  selection. Timed at 1,000 names: a filter keystroke; create; update;
  delete.
- **Circle Drawer**: click to add a circle, the nearest circle under
  the pointer highlighted, a right-click dialog with a diameter slider
  whose adjustment is one undoable change; undo and redo. Timed at
  1,000 circles: add; a diameter change; undo; redo; the hover
  highlight as the pointer moves.
- **Cells**: a spreadsheet of rows 0 to 99 and columns A to Z, cells
  holding formulas, only dependants recomputed on change. Timed: one
  edit at the head of a 100-cell dependency chain, to the frame that
  shows the chain's last cell.

What each tests of a framework: text input and its validation
(Temperature, Flight Booker, CRUD), a running animation and idle
(Timer), a filtered list (CRUD), custom drawing with hit-testing and
undo (Circle Drawer), and change propagation through a dependency
graph (Cells). Slint ships its own 7GUIs examples; iced and egui have
community ones (3.5 to 3.7); GPUI and Blitz have none found.

## 2. The method: what is measured and how

### 2.1 One definition of an operation's time

Upstream times from the click's dispatch to the end of the paint that
follows it (1.1). The native equivalent, used for every framework here:

- **Start**: the harness hands the framework the input event (a pointer
  press and release at the control's position, or a key) through the
  framework's own event entry point (3.1). Nothing before that is
  timed: building the event, deciding where to click, reading the
  spec.
- **End, CPU**: the framework has produced the frame that shows the
  result and asks for no further immediate frame. The harness calls
  the framework's frame function in a loop until the framework reports
  idle (3.1, `Driver::frame` returns `Idle`), so a framework that needs
  two passes for one change (an immediate-mode layout that sizes on the
  second pass, a retained one that schedules a follow-up redraw) is
  timed for both, and the number of frames is recorded per operation.
  This is the one-scene plan's frame-to-idle (`docs/plans/one-scene.md`,
  D6) applied to an operation.
- **End, GPU**: the GPU has finished every command the operation
  submitted. The harness waits on the device after the last frame, as
  `vapors-perf` does between frames (`sampler.rs`, `FRAME_DEADLINE`),
  and times that wait separately.

Three numbers per operation, then:

| number | from | to | comparable across frameworks |
|---|---|---|---|
| CPU time | the event is handed over | the frame function returns idle | yes: it is what the harness itself called |
| to GPU done | the same | the device's wait returns | yes: every backend can be waited on (a wgpu poll, a Vulkan fence, `glFinish`) |
| GPU time | the first render pass's start | the last pass's end, on the GPU's clock | only where the framework exposes its wgpu queue (3.2 to 3.7 say which); shown as "not measured" elsewhere, never as 0 |

CPU time is the scored number (5). "To GPU done" shows what a window
would have waited for and catches a framework that returns before it
has submitted. Phase breakdowns (`vapors-perf`'s `style`, `layout`,
`paint`, `draw`) exist only for vapors and are published only in the
vapors run's own appendix as explanation, never in the comparison
table: the other frameworks have no spans that mean the same things.

Idle is a framework's own statement (iced's and egui's repaint requests,
Slint's and GPUI's redraw scheduling, Blitz's and vapors' `Next`), with
a deadline: an operation still asking for frames after 60 frames is
refused, naming the framework and the operation, since the result
cannot then be told from an animation. The 60 is one second of a 60 Hz
clock; a timed operation here is expected to settle in one to three
frames, and the slice that first runs each framework records what it
needed (7, B0 to B5).

### 2.2 Timing, instruction counts and spread

`vapors-perf`'s method (`docs/perf/baseline-2026-11-10.md`, Setup and
Instruction counts), applied to every framework:

- **Release build**; a debug build refuses to run, as `vapors-perf`
  does (`main.rs`).
- **Warmup, then samples.** Each sample is one operation on a fresh
  pre-state (1.2 says what each operation's pre-state is and how it is
  restored, untimed, between samples). 5 untimed, then 30 timed, for the
  operations on 1,000 rows; 3 and 10 for create 10,000, as
  `vapors-perf` caps its 10,000-row workloads (`workloads.rs`,
  `BIG_PAGE_SETTINGS`). Upstream's counts are in 1.1 for comparison;
  the reason not to copy them is 2.1's end condition, which makes each
  sample dearer than a trace.
- **Statistics**: median, p90 and max by nearest rank, never a mean
  (`stats.rs`). The published figure is the median; p90 and max are in
  the per-run files (5).
- **Three runs**, each run a new process per framework (6), in
  rotating order (run 1: vapors first; run 2: vapors last; run 3: the
  middle) so thermal state and page cache favour no framework. The
  published median is the median of the three runs' medians, with the
  spread (largest over smallest of the three) beside it. A spread over
  10% is printed in the table as it is and the framework's row is
  marked "unstable", not dropped; 10% is read from the wall-time spread
  `vapors-perf` measured on a loaded machine (11.6% to 16.9%,
  `baseline-2026-11-10.md`), which a quiet machine should beat, and the
  first full run (7, B6) replaces it with a measured figure.
- **Load limit**: a run refuses to start when the 1-minute load average
  is 1.5 or more (the finish line's limit), and refuses its result when
  it is at the end; other agent work is paused for a run (`docs/perf/`
  records load at both ends).
- **Instructions** retired, from perf events opened in the process
  (`instructions.rs`), as the second table: the count moves with the
  code, not with the machine (0.06% spread against 44% for wall time,
  `baseline-2026-11-10.md`). One difference from `vapors-perf`: the
  counter is opened with `inherit` before the framework's application
  is made, so threads the framework spawns afterwards (GPUI's
  background executor, Blitz's and Vello's workers, a renderer's
  transfer thread) are counted too; `vapors-perf` counts the calling
  thread alone. Whether a `read` of an inherited counter sums live
  child threads on this kernel is not verified; B0 proves it with a
  thread that spins a known loop, and if it does not, the count stays
  per calling thread and the table says so for the frameworks that use
  other threads.
- **Sizes**: upstream's (1,000 rows for update, select, swap and
  remove; 1,000 and 10,000 for create). D3 asks whether the four edits
  are also run at 10,000 rows.

### 2.3 Offscreen, not a window

Every framework that can draws into a texture the harness owns, with no
surface, no present and no vsync, as `vapors-perf` does
(`baseline-2026-11-10.md`, Target: offscreen). The reasons:

- The dev machine has no display. Xvfb gives an X server with no
  compositor and no vsync, so a present there is a copy whose cost
  varies by driver and WSI path; it would be in some frameworks' numbers
  and not others'.
- The window system's event loop is out of the measurement for every
  framework. Injecting X events into a real window (XTest) and reading
  the result back from the server would add the server's round trips,
  milliseconds against operations expected under one, and would make
  the end condition a screenshot poll.
- Each framework already has a layer below its window that its own
  tests drive this way (3.2 to 3.7: `vapors-perf`'s `App::frame`, egui's
  `Context::run` with `egui-wgpu`, iced's runtime and its test crate,
  Slint's software renderer and testing backend, Blitz's document and
  `anyrender`).

A framework with no offscreen path (GPUI is the candidate, 3.3) runs in
a window under `xvfb-run`, and its number includes whatever its present
costs there; the table says so in that row, and D1 is the decision on
whether to run every framework windowed instead so the rows match.

Pixels are read back after the last frame of each check (2.4), untimed,
by a copy of the texture to a mappable buffer (`vapors-pixels` does this
for vapors' screenshots).

### 2.4 The harness's own checks

A run that did not perform the operation is refused, exiting non-zero
and naming the framework, the operation and the sample. The checks:

- **The result is on screen.** After the operation's last frame the
  harness reads the texture and checks the pixels the spec (6) predicts
  for that operation's post-state, in regions that hold no text, so
  that text rendering differences between frameworks do not matter:
  - create 1,000 and 10,000: the row background bands count to the
    number of rows the viewport shows, and the row below the last is
    the page background;
  - update every 10th: the label cell of rows 1, 11, 21 differs from
    its pixels before the operation, and rows 2 and 12 do not (region
    hashes of the same framework's own rendering, before and after);
  - select: row 2's background is the selected colour, and row 1's and
    row 3's are not;
  - swap: row 2's label cell now hashes to what row 999's did, and row
    999's to what row 2's did; row 999 is below the viewport, so the
    check scrolls the list to the end after the timed operation,
    reads, and scrolls back, all untimed (D4 is whether to check only
    what is visible instead);
  - remove: the row count is one fewer (bands), and row 4's label cell
    hashes to what row 5's did.
  A check that cannot be made (a framework whose row height is not the
  spec's, so the bands are not where expected) refuses the run naming
  the mismatch, not the framework; the app is wrong, not the check.
- **The operation was done once.** The driver counts the framework's
  frames per sample; more than one per operation is recorded, more than
  60 refused (2.1).
- **The pre-state was restored.** Before each sample the same pixel
  checks run on the pre-state (1,000 rows, nothing selected).
- **No GPU work was left.** The device wait after the last frame has a
  deadline (10 s, `vapors-perf`'s) and a failure names it; a frame
  whose GPU time is missing where the framework provides it is refused,
  never counted as 0.
- **Build and environment.** Debug build; load over the limit at start
  or end; a counter that was multiplexed, or fails its self-check
  (`instructions_check.rs`); a framework crate at a version other than
  the lock file's (the run prints every framework's resolved version
  and compares it with the pins file, 6).
- **The check can fail.** Each check is proven in its slice by making
  the app skip the operation (an app flag the harness does not know)
  and watching the refusal, then restoring; the commit says so.

## 3. Driving each framework

### 3.1 Common shape

One application per framework for the rows page, and one per framework
per 7GUIs task, each a binary in `vapors-bench` (6), each implementing
one trait the harness drives:

```rust
pub trait Driver {
    /// Make the application at the spec's size on this device; a
    /// framework that needs a window says so and the runner starts it
    /// under Xvfb (D1).
    fn new(gpu: &Gpu, spec: &Spec) -> Result<Self, String>;
    /// Hand the framework one input event: pointer press and release
    /// at a point, or a key; untimed set-up is the caller's.
    fn input(&mut self, event: Input);
    /// One frame into `texture`; `Idle` when the framework asks for no
    /// immediate frame, `Again` when it does.
    fn frame(&mut self, texture: &wgpu::Texture) -> Result<Next, String>;
    /// The wgpu queue the frame submitted on, for GPU time; `None`
    /// where the framework does not render through wgpu.
    fn queue(&self) -> Option<&wgpu::Queue>;
    /// The model's view of the page, the weaker check (2.4).
    fn rows(&self) -> usize;
}
```

The harness times `input` plus the `frame` loop to `Idle` (2.1). The
input's position comes from the spec: every application lays the page
out at the spec's coordinates, so "row 2's label" is one point for all.
Pinned versions live in one file, `pins.toml`, which the runner reads
and compares with `cargo metadata`'s resolved versions before anything
runs (2.4).

Not shared: no common widget code, no common model. Each application is
its framework's own, by rule 4.

### 3.2 vapors

- **Pin**: a git SHA in `.config/vapors-rev`, the merge of the finish
  line's last property.
- **Driven**: as `vapors-perf` drives its rows workload: an
  `ElementsApp` over a compiled `.wisp` page, `App::frame` on the
  harness's texture, input through the app's event entry (what the
  platform crate hands it), `Next` as idle. GPU time from the wgpu
  queue. The rows page is written fresh from vapors' docs for the
  spec, not `crates/perf/ui/mount_rows.wisp`, which was written for
  the finish line's measurements and is not a user's application
  (rule 5).
- **Not special**: the harness reads nothing from vapors the other
  drivers cannot give; the phase spans are collected only for the
  appendix (2.1).

### 3.3 GPUI

- **Pin**: D9. The crates.io `gpui` is 0.2.2 (2025-10-22; Apache-2.0)
  and renders on Linux through blade-graphics 0.7 over Vulkan. Zed's
  repository replaced blade with wgpu on Linux in PR #46758 (merged
  2026-02-13), so the published crate is a year behind what Zed ships
  and differs in its renderer; GPUI makes no stability promise and its
  setup docs cover macOS only.
- **Architecture**: retained; state in `Entity<T>`, views are entities
  implementing `Render`, re-rendered after `cx.notify()` (not
  verified against the source); layout by taffy (pinned `=0.9.0`),
  text by cosmic-text on Linux; Linux windowing is GPUI's own X11
  (x11rb) and Wayland clients, not winit. Lists: `uniform_list`
  (virtualised, equal heights) or `list` with a `ListState` (D2).
  Text input is a custom element in GPUI; 7GUIs' fields are built from
  Zed's own input example.
- **Driven**: the published crate's `TestAppContext` simulates input
  on a test platform and does not render, so it cannot time a frame.
  Zed's main has a `HeadlessAppContext` with `capture_screenshot` and a
  `VisualTestAppContext` (PR #45259, macOS and Metal first); whether
  either renders on Linux with wgpu is not verified. The two routes G0
  tries, in order: (a) the headless context on a Zed git pin, which
  would make GPUI an offscreen row; (b) a real window under Xvfb
  (`DISPLAY` set, no `WAYLAND_DISPLAY`), the frame driven by the
  platform's own loop with the harness's input delivered through
  GPUI's test-support simulation if it reaches a windowed app, or else
  through XTest, which is 2.3's worst case. `ZED_HEADLESS` selects a
  headless platform in Zed's Linux code (seen in a search result, not
  verified); it may be what (a) needs.
- **Idle**: no pending `notify` and no requested frame; how GPUI
  exposes that outside its loop is G0's finding.
- **7GUIs**: no implementation found.

### 3.4 Blitz

- **Pin**: `blitz-dom` 0.3.0-beta.4 (crates.io, 2026-10-16; MIT or
  Apache-2.0) with `dioxus-native` 0.8.0-alpha.3; or a git SHA if B4
  needs the WPT runner's unreleased pieces. The README calls Blitz beta
  with "numerous defects and unimplemented functionality"; its CSS
  status page lists 3D transforms, `overflow: auto`, `mix-blend-mode`,
  sticky positioning and container queries as missing. None of those
  is in the rows page.
- **Architecture**: a retained HTML and CSS DOM: Stylo for style,
  Taffy for layout, Parley for text, in `blitz-dom`; `blitz-paint`
  turns the tree into draw commands; `blitz-shell` is winit, AccessKit
  and muda. Drawing goes through AnyRender with backends
  `anyrender_vello` (wgpu), `vello_hybrid`, `vello_cpu` and skia. Two
  ways to write an application: the `blitz` crate renders an HTML
  string without interaction; `dioxus-native` runs a Dioxus component
  tree and applies its mutations to the DOM. The idiomatic application
  is a Dioxus one on `dioxus-native` (signals holding the rows, keyed
  rows; no virtualisation built in), since that is what Blitz exists to
  render; mutating `blitz-dom` directly is the second route if B4 finds
  `dioxus-native` cannot be driven without `blitz-shell`.
- **Driven**: Blitz's WPT runner renders a document offscreen at
  800 x 600 through `VelloImageRenderer` or `VelloCpuImageRenderer` to
  bytes, and that is the pattern to copy: there is no stable "set DOM,
  lay out, paint to image" API, and the runner's set-up is the public
  example of it. Input: `blitz-dom`'s event handling (`DomEvent` for
  pointer and keyboard; not verified to be public API at the pin).
  Vello on wgpu gives GPU time; Vello on ANV and lavapipe is not
  verified and is B4's first finding; `vello_cpu` is the fallback and
  a software row (rule 2).
- **Idle**: the document reports no pending changes and the Dioxus
  scheduler has no dirty scopes.
- **7GUIs**: no implementation found; written from Dioxus's docs.

### 3.5 iced

- **Pin**: `iced` 0.14.0 (crates.io, 2025-12-07; MIT). 0.15 exists only
  as `-dev` on the repository and is not pinned.
- **Architecture**: Elm-style; the application's `update` takes a
  message and the whole `view` is rebuilt and diffed against the
  retained widget tree on each update; layout is iced's own; text by
  cosmic-text; rendering by `iced_wgpu`, with `iced_tiny_skia` as the
  software fallback (`ICED_BACKEND=wgpu|tiny-skia`). tiny-skia has
  damage tracking; the wgpu renderer redraws the frame. No virtualised
  list in iced itself; `keyed_column` is the keyed list, as upstream's
  keyed rule wants (1.1), so the application materialises every row.
- **Driven**: `iced_test` (the test crate shipped with 0.14): a
  `Simulator` that builds the user interface for a size, takes
  `click`, `tap_key` and `typewrite`, finds widgets, and renders a
  `snapshot`/`screenshot` to pixels through the renderer; there is also
  an `Emulator`. The driver builds the simulator on the wgpu renderer
  (so GPU time is readable, rule 2) with the spec's size; whether
  `iced_test` runs the wgpu renderer rather than tiny-skia for its
  snapshots is not verified and is B2's first finding. If it is
  tiny-skia only, the driver uses `iced_runtime`'s `UserInterface`
  with `iced_wgpu` directly, which is what `iced_test` wraps.
- **Idle**: a simulator step with no message produced and no redraw
  requested.
- **7GUIs**: an existing iced implementation,
  github.com/dcampbell24/iced_7guis, is a reference for rule 4's
  review, not code to copy (it is not iced's own example).

### 3.6 Slint

- **Pin**: `slint` 1.18.1 (crates.io, 2026-09-21; GPL-3.0-only, or the
  Royalty-free 2.0 or Software 3.0 licences; the royalty-free licence
  is the one a benchmark application uses, and rule 1 cites it).
- **Architecture**: `.slint` markup compiled to Rust by `slint-build`;
  a retained element tree with reactive properties, re-evaluated on
  change; layout its own; backends `winit`, `qt` and `linuxkms`;
  renderers `software` (with partial redraw), `femtovg` (OpenGL),
  `skia` and `vello`, chosen by feature and `SLINT_BACKEND`
  (`winit-femtovg`, `winit-skia`, `winit-software`). `ListView`
  virtualises (D2 applies; its materialisation count is published).
- **Driven**: `i-slint-backend-testing` (the testing backend):
  `init_no_event_loop`, `ElementHandle` to find elements and send
  clicks and key sequences, `mock_elapsed_time`; `SLINT_BACKEND=
  headless` runs with no window. Whether the testing backend renders
  at all, and with which renderer, is not verified: Slint's offscreen
  rendering is the software renderer's (it renders to a pixel buffer
  the caller owns, through `slint::platform` and a custom platform).
  B3 decides between: (a) the software renderer, a different renderer
  from the GPU ones (rule 2 marks the row, no GPU time); (b) a window
  under Xvfb with skia on Vulkan if Slint's skia feature offers Vulkan
  on Linux (not verified), which is D1's windowed exception. Rule 2
  and the risk in 9 say what each means for the row.
- **Idle**: no pending property change and no redraw requested after
  `mock_elapsed_time(0)`.
- **7GUIs**: Slint's own `examples/7guis` in its repository: the
  reference for rule 4, and the idiomatic source for 7G0 to 7G5.

### 3.7 egui

- **Pin**: `egui` 0.37.0 (crates.io, 2026-10-15; MIT or Apache-2.0),
  with `egui-wgpu` for the renderer; `eframe` is not needed offscreen.
- **Architecture**: immediate mode; the whole UI function runs every
  frame, producing shapes that `epaint` tessellates; repaint is on
  demand (an idle app draws no frame) but a frame is the whole frame;
  layout is its own, text its own with `epaint`'s font atlas. Lists:
  `ScrollArea::show_rows` virtualises rows of uniform height, which
  the docs give as the way to show many rows (D2; materialisation
  published); a plain loop in a `ScrollArea` materialises every row
  and is egui's other documented shape.
- **Driven**: `egui::Context::run` takes `RawInput` (events, time,
  screen rect) and returns `FullOutput` with the shapes and the
  repaint request; `egui_kittest` (egui's own test harness) does this
  and renders snapshots with `egui-wgpu` under its `wgpu` feature, and
  finds widgets through the AccessKit tree. The driver uses `Context::
  run` and `egui_wgpu::Renderer` directly, the plainest offscreen path
  in the set, with GPU time from the wgpu queue. Multi-pass: egui may
  ask for a second pass in the same frame (`request_discard`) when a
  layout needs sizing; both passes are inside the timed span and the
  count is published (2.1).
- **Idle**: `FullOutput`'s viewport output has no repaint request
  with zero delay.
- **7GUIs**: no official egui implementation; a community one exists
  (github.com/rust-ninja-sabi/rust-egui-seven-tasks, coverage not
  verified). The reference for rule 4 is egui's own widget gallery and
  `ScrollArea` docs.

## 4. Fairness rules

1. **One build profile.** Every application is built by one
   `[profile.release]` in the workspace (`opt-level = 3`, `lto = "fat"`,
   `codegen-units = 1`, `debug = false`, the default `panic`), printed
   on the results page. A framework's docs that recommend a different
   setting for release (a feature flag, an allocator) get it, cited;
   nothing else is tuned.
2. **One GPU driver, one renderer class.** Everything that can runs on
   Vulkan through Mesa ANV on the UHD 630, through wgpu where the
   framework uses wgpu and through its own Vulkan path otherwise
   (GPUI's blade at the crates.io pin, or wgpu at a Zed git pin, D8).
   A framework whose only Linux renderers are OpenGL or
   software (Slint's choices, 3.6) runs on the nearest of them, and its
   row names the renderer. Software rasterisers (iced's tiny-skia,
   Slint's software renderer, Vello CPU) are not scored rows; one may
   be published as a second, marked row where it is the framework's
   documented default.
3. **One page.** Every application draws the spec's page (6): the same
   viewport (640 x 400 at scale 1, `vapors-perf`'s rows size, or wider
   if D4 needs more rows visible), the same row height, column
   positions, colours and font file, so the checks (2.4) and the eye
   see the same thing. The font is one file bundled in the repository
   and loaded into each framework by its documented route, so text
   shaping covers the same glyphs. Where a framework cannot take a
   font file (not expected; 3.x says if found), its row says which
   font it used.
4. **Idiomatic, from the docs.** Each application is written from the
   framework's own documentation and examples at the pinned version,
   and its README names the examples it follows. The implementer does
   not read vapors' application first. A review agent with no author
   context reviews each application against the same docs with one
   question: would the framework's own examples write it this way? Its
   findings are fixed before the application's first timed run.
   Virtualised list widgets are allowed where they are the framework's
   documented list (D2), and the results table says per framework how
   many rows it materialises per frame.
5. **vapors gets the same treatment.** The vapors application uses
   only `vapors`' public, documented API at the pinned SHA. A vapors
   change motivated by a benchmark result lands in vapors `main` through
   its own PR with its own tests, and the benchmark re-pins; no fast
   path exists for the benchmark alone. The review in rule 4 reads the
   vapors application too.
6. **Authors invited.** Before results are published with a framework's
   name, an issue is opened on `vapors-bench` per framework (never on
   the framework's own tracker, which is for its bugs: memory
   `upstream-issues-not-prs`), titled "Is this how you would write it?"
   and linking the application; a correction arrives as a PR to
   `vapors-bench`, is reviewed by rule 4's question, and a re-run with
   it replaces the published numbers with a line in the page's
   changelog naming the correction and the change it made. D6 is how
   far the invitation goes beyond the issue (a post in each
   framework's own forum is outward-facing; Tom decides).
7. **Nothing hidden.** Every operation runs for every framework, or the
   table says "not run" with the reason and a link to the issue (5). A
   result is never left out because of what it shows.

Published as context, not scored: binary size of each application
(release, stripped), resident memory after create 1,000 and after
create 10,000 (`RssAnon`, as `mount-2026-11-12.md` reads it), time to
first frame of the empty page, and each application's line count.

## 5. What is published

`results/<date>-<run>/` in the repository (6) holds the raw per-run
files, one JSON per framework per run: the environment line (CPU, GPU
and driver versions, kernel, rustc, the framework's version or SHA, the
harness's SHA, load at start and end), and per operation every sample's
CPU time, to-GPU-done, GPU time or `null`, frames, instructions, and the
checks' outcomes. `results/<date>/README.md` is generated from them by
a script in the repository (never by hand or by a model) and is what
the website later shows:

- One table per operation and size, frameworks sorted by median CPU
  time, fastest first, so vapors sits wherever it measured. Columns:
  median CPU µs, p90, spread over three runs, to-GPU-done median, GPU
  time median or "not measured", frames per operation, instructions
  (median), rows materialised. Every row the same style; nothing is
  bold or coloured for vapors.
- A "where vapors is not fastest" section, generated: each operation
  where another framework's median is lower, the framework, and the
  ratio. It is the first section after the tables. The symmetric
  "where vapors is fastest" section follows it, same format.
- One summary number per framework, upstream's (1.1): the geometric
  mean over operations of the framework's median divided by the
  fastest median for that operation, labelled as that and placed after
  the two sections, not before (D5 is whether to publish it at all).
- "Not run" cells with the reason and the issue link; a framework with
  any such cell has no summary number, since a mean over fewer
  operations is not the same number.
- The context figures of section 4 in their own table.
- The changelog: every correction accepted under rule 6, and every
  re-pin, with what moved.
- The method, in one page: sections 2 and 4 restated, with the exact
  command that reproduces a run.

Each framework's application has a README with the examples it follows
and anything it could not do the spec's way.

## 6. Repository and layout

A separate repository, `vapors-bench` (`~/projects/vapors-bench`, the
same worktree and target conventions as vapors and touche), so its
dependency set (five frameworks and their transitive crates) never
enters vapors' `Cargo.lock`, `cargo deny` policy or build times, and so
its pins move on their own schedule. It depends on vapors by git SHA,
recorded in `.config/vapors-rev` as touche does, and on the other
frameworks by crates.io version (Blitz by git SHA if it is not on
crates.io at the time, 3.4), with `Cargo.lock` committed. Crates:

- `bench-harness`: the `Driver` trait (3.1), the sampler, the pixel
  checks, the perf counter with `inherit`, the device wait, and the
  JSON writer. The statistics, the counter's self-check and the GPU
  clock come from `vapors-perf` as a library (`vapors_perf::stats`,
  `instructions_check`, `gpu_time`), by the same SHA pin, so the method
  has one implementation; what `vapors-perf` does not export yet is
  exported in a small vapors PR first (7, B0's dependency).
- `bench-spec`: the page spec as data (row height, column x positions,
  colours, viewport, font path), read by every application and by the
  checks, so one source answers both (CLAUDE.md, "one question one
  answer"), and a unit test that the check positions derive from it.
- `app-vapors`, `app-gpui`, `app-blitz`, `app-iced`, `app-slint`,
  `app-egui`: one binary each, implementing `Driver` for the rows page;
  later the 7GUIs tasks as further binaries in the same crates
  (`app-egui/src/bin/counter.rs`, ...).
- `bench-run`: the runner. It starts each application as its own
  process (a crash or a leaked allocator in one never reaches another;
  GPUI's loop can own its main thread), under `xvfb-run -a` where the
  application declares it needs a display (3.1), collects the JSON, and
  refuses the run on any refusal from any application.
- `scripts/report`: the results README generator (5), with a test on a
  fixture results directory.

CI builds every crate and runs the unit tests (spec derivation, the
report generator, the pixel-check functions on synthetic images) and
times nothing, as vapors' CI does not (`baseline-2026-11-10.md`, "never
asserted in CI"). Timed runs are on the dev machine, by hand, with
other agents paused, and committed with their results directory.

## 7. Slices

Each under about 400 lines (estimate), built by a fresh agent, every
check proven by breaking what it guards. After each slice the next is
checked against what it found. The first slice is one operation on two
frameworks: create 1,000 rows, on vapors and egui. Create first because
every other operation's pre-state is its result; egui second because it
is the framework furthest from vapors in architecture (immediate mode,
no retained tree) and the one with the plainest offscreen path (3.7),
so the harness's shape is settled against the hardest case for its
`Driver` trait before the retained frameworks follow.

| Slice | Content | Accepted when | Depends on | Parallel with |
|---|---|---|---|---|
| V0 | vapors PR: `vapors-perf` exports `stats`, `instructions_check` and `gpu_time` as library API (they are `pub mod` already; what is missing is a `[lib]` the bench can depend on without the binary's `main.rs` deps), and `vapors-pixels`' read-back as a function | `cargo xtask ci` green; a doc test using the three from outside the crate | none | everything |
| B0 | `vapors-bench`: the repository, the spec crate, the `Driver` trait, the sampler (2.1, 2.2), the perf counter with `inherit` and its thread test, the pixel-band and region-hash checks on synthetic images, the JSON writer; `app-vapors` and `app-egui` with create 1,000 and its check; `bench-run` for two processes | Both applications run create 1,000 thirty times under the harness and pass the band check; each refusal in 2.4 shown failing (the app skips the create; a frame reports before its GPU wait; a debug build) then restored; the `inherit` test's outcome recorded in 2.2 | V0 | G0 |
| G0 | GPUI spike: a GPUI window under `xvfb-run` on this machine draws the spec's empty page on blade over Vulkan, the harness reads its pixels, and one click reaches a handler; or a written record of what stopped it (3.3, 9) | The pixels read back match the spec's background, and the click's handler ran (a counter in the app the harness reads), or the record names the failing layer (Vulkan WSI on Xvfb, blade's adapter choice, GPUI's X11 client) with the command | none | B0 |
| B1 | The other five operations on vapors and egui: update every 10th, select, swap (with D4's scroll check), remove, create 10,000; the pre-state restore; the results README generator with a fixture test | Every check in 2.4 shown failing per operation, then restored; one results directory committed from one run of the two frameworks, generated README included | B0 | B2 to B5 after their drivers exist |
| B2 | `app-iced`: the driver (3.5) and the six operations | B1's checks pass on iced; its README names the examples followed; the rule 4 review done | B1 | B3, B4, B5 |
| B3 | `app-slint`: the driver (3.6), the renderer choice recorded, the six operations | The same | B1 | B2, B4, B5 |
| B4 | `app-blitz`: the driver (3.4), the six operations | The same; Blitz's pin recorded (crates.io version or SHA) | B1 | B2, B3, B5 |
| B5 | `app-gpui`: the driver as G0 found it (3.3), the six operations | The same, or G0's record and "not run" cells with it | G0, B1 | B2, B3, B4 |
| B6 | The first full publication: three runs on the dev machine in rotating order, spread measured and 2.2's 10% replaced, the context figures (4), `results/<date>/README.md`; the six rule-6 issues opened | Every cell filled or "not run" with its issue; the "not fastest" section present even when empty; the method page reproduces the run from its command | B2 to B5 | 7G0 |
| 7G0 | 7GUIs Counter and Temperature Converter on all six: keyboard injection in every driver, a text field per framework, the operations "one click to the frame" and "one keystroke to the converted value on the frame" | Checks: the count's digit cell changed; the converted field's cell changed and the invalid-input colour appears for a letter; each refusal shown failing | B2 to B5 | B6 |
| 7G1 | Timer: the running animation; measured as CPU per frame over 5 s of the frame clock, and idle frames after the duration slider is dragged to zero | Per-frame CPU median per framework; a framework drawing frames after the timer stops is recorded as such (not refused: egui draws while a slider is hovered by design, and that is a result) | 7G0 | 7G2, 7G3 |
| 7G2 | CRUD at 1,000 names: a filter keystroke, create, update, delete | The filtered list's band count matches the names that match; each operation's check | 7G0 | 7G1, 7G3 |
| 7G3 | Circle Drawer at 1,000 circles: add a circle, adjust its radius through the dialog, undo, redo; hover highlight | Pixel checks on the circle's fill at its centre; undo restores the prior pixels | 7G0 | 7G1, 7G2 |
| 7G4 | Cells: a 26 x 100 sheet with a 100-cell dependency chain; one edit at the chain's head | The chain's last cell shows the new value (its digit cell changed); frames per operation recorded (a framework that propagates over several frames is a result) | 7G0 | 7G5 |
| 7G5 | Flight Booker: the date validation keystroke and the combo change | The invalid colour and the disabled button's colour | 7G0 | 7G4 |
| B7 | Second publication with 7GUIs; D3's 10,000-row edits if taken; the website's data source agreed with the site plan | As B6 | 7G1 to 7G5 | none |

Critical path: V0, B0, B1, then B2 to B5 in parallel, B6; 7G0, then
7G1 to 7G5 in parallel, B7. G0 can start now, with nothing else, and
decides D1 before B5.

## 8. Decisions

Each with a recommendation; sections 1 to 7 follow the recommendation
unless the decision says what changes.

**D1. Windowed or offscreen.** (a) Offscreen for every framework that
can be (2.3); windowed under Xvfb only for a framework with no
offscreen path, its row marked. (b) Windowed under Xvfb for every
framework, so every row includes a present and the window system's
input path. (c) Offscreen only; a framework with no offscreen path is
"not run" with the reason. Recommendation: (a). (b) puts a cost that
varies by driver and WSI path into every number and makes the end
condition a screenshot poll (2.3); (c) leaves GPUI out of a benchmark
Tom named it for. Under (a) one row may be less comparable; the row
says so and G0 measures how much the present costs on this machine
(the same page drawn windowed and offscreen by vapors, which can do
both).

**D2. Virtualised lists.** (a) Each framework's documented list widget,
virtualised or not, with rows materialised per frame published beside
the number. (b) Every application materialises every row (no
virtualisation), as upstream requires of DOM implementations (1.1).
(c) Both, as two rows per framework where the framework's docs show
both. Recommendation: (a). The benchmark is of what the docs lead a
user to write; a framework whose idiomatic list virtualises has made
that choice for its users, and the published materialisation count says
what the number means. (b) would publish a GPUI, Slint or egui
application their authors would not write. (c) doubles B2 to B5 for the
frameworks it applies to and can be added later per framework if a
result is disputed (rule 6).

**D3. The edits at 10,000 rows.** (a) Upstream's sizes only: edits at
1,000, creates at 1,000 and 10,000. (b) The four edits also at 10,000
rows, as a separate table labelled "not an upstream operation".
Recommendation: (b). The finish line's claim is that an edit costs the
same at 1,000 and 10,000 rows (property 1); the comparison is where
that claim is tested against others, and the label keeps the upstream
set clean. Cost: the pre-state restore at 10,000 rows makes those
samples slower to run, not more code.

**D4. Checking off-screen rows.** (a) The swap and remove checks
scroll to the end, untimed, to read row 999 (2.4). (b) Check only the
visible rows (row 2's cell changed to row 999's prior hash; the count
after remove) and read row 999 from the application's own state. (c) A
viewport tall enough to show 1,000 rows at the spec's row height (no:
24,000 px is over the texture limit on every backend here). Recommendation:
(a). (b) asks the framework whether it did the operation, which is the
model, not the screen; the check is of what was drawn. (a) needs a
programmatic scroll in every driver, which 7GUIs needs anyway.

**D5. The summary number.** (a) Publish a geometric mean of the
ratios to the fastest per framework, unweighted, after the
per-operation sections (5). (b) Per operation only. (c) Upstream's
weights (1.1). Recommendation: (a). Readers who know upstream expect a
geometric mean, and placing it after the "not fastest" section keeps
the losses first; the number's definition is on the page. Not (c):
upstream's weights are derived from the spread across web frameworks
on its nine operations, and this set is six operations on native
frameworks, so the weights would be numbers from another table.

**D6. How far the author invitation goes.** (a) An issue on
`vapors-bench` per framework, linked from the results page, and
nothing posted elsewhere. (b) (a) and one post per framework in its
own community forum or chat, linking the issue, before the first
publication. Recommendation: (b), with the post's text drafted for
Tom's approval in B6: it is outward-facing in his name, and an
invitation nobody sees is not one. If Tom prefers (a), rule 6 stands
with the issues alone.

**D7. Second framework in B0.** (a) egui, for the reasons in 7. (b)
iced, the framework most like vapors in shape (retained Elm-style tree,
wgpu), so B0's harness is checked on the easy case first.
Recommendation: (a): a trait that fits both vapors and egui fits the
rest; one that fits vapors and iced may not fit egui, and B1 would then
reshape it.

**D8. Which GPUI.** (a) The crates.io `gpui` 0.2.2: blade on Vulkan,
a year behind Zed, the version a third party gets with `cargo add`.
(b) A git SHA of Zed's repository at a Zed release tag: wgpu on
Linux, the renderer Zed ships, and the headless context (3.3) if it is
there, pinned like vapors is. Recommendation: (b). The benchmark
names GPUI as a framework a user would pick; what Zed ships is what
that user runs, and (a) would publish a renderer GPUI has since
replaced. The row names the tag and SHA. If G0 finds the git pin does
not build outside Zed's workspace (its path dependencies), (a) is the
fallback and the row says so.

**D9. Which 7GUIs tasks are timed.** (a) All seven, as 7G0 to 7G5 lay
them out, with Counter, Temperature Converter and Flight Booker as
single-operation tasks. (b) Timer, CRUD, Circle Drawer and Cells only
(the four with a size parameter), the other three built as code
samples and not timed. Recommendation: (a): once a task's application
exists in every framework, timing its one operation is one driver
call, and the keyboard path 7G0 builds is needed by the four anyway.

## 9. Risks

- **GPUI on Linux.** GPUI's Linux platform is X11 and Wayland clients
  of Zed's own, rendering through blade on Vulkan at the crates.io pin
  and wgpu at a Zed git pin (3.3, D8); its published test context does
  not render, and the headless context that does is unreleased and
  macOS-first. On Xvfb there is no DRI3, so a Vulkan swapchain on ANV
  may fail to create and lavapipe's X11 presentation path is the
  fallback; whether the renderer picks it, and whether GPUI's X11
  client needs anything Xvfb lacks (XInput 2, a clipboard manager), is
  not verified. A git pin of Zed may not build outside Zed's workspace.
  G0 exists to find all of this out before anything depends on it; its
  failure mode is a "not run" row with the record, not a blocked plan.
- **Blitz maturity.** Blitz describes itself as experimental (3.4); its
  crates.io releases may lag its repository, text input and scrolling
  may be incomplete for 7GUIs, and its API may move between B4 and B7.
  Pin by SHA where needed; a 7GUIs task Blitz cannot build is a "not
  run" cell with the issue, and the rows page is expected to work
  (it is static HTML with an edit path).
- **Offscreen paths that are not public.** iced's and Slint's offscreen
  routes (3.5, 3.6) are their test crates' and may not expose a wgpu
  queue for GPU time or may skip steps a window does (Slint's software
  renderer against its GPU renderers is a different renderer, not the
  same one offscreen). The row names what ran; GPU time is "not
  measured" where it cannot be read, never estimated.
- **The dev machine's GPU.** One integrated GPU, Mesa ANV, shared with
  every other agent's GPU tests. Timed runs need other work paused, as
  the finish line's runs did; the load limit refuses otherwise. A
  second machine would make results more general and is not planned
  (`tom-business-context`: a Mac Studio once revenue allows).
- **Five idiomatic applications, reviewed.** Six applications for the
  rows page and up to forty-two for 7GUIs, each reviewed against its
  framework's docs. The slices put the rows page first and publish it
  (B6) before 7GUIs starts, so a first publication does not wait on the
  forty-two.
- **The harness's own cost.** The event injection, the idle poll and
  the pixel read are outside the timed span by construction (2.1), but
  the driver's frame loop is inside it: a driver that does more per
  frame for one framework than another (converting events, copying a
  texture) biases that row. Every driver's `frame` is reviewed to do
  only what the framework's own windowed loop would, and B0 times an
  empty driver (no framework) as the floor, published.
- **Version churn.** iced's and Blitz's releases move; a re-pin changes
  numbers. Every re-pin is a changelog line with the before and after
  (5), and the pins file is the one place versions live.
- **Reading an upstream number into this one.** Readers will compare
  with the js-framework-benchmark table. Section 1.1 and the method
  page say why that is not meaningful (a trace inside Chrome against a
  native frame loop; different end conditions; different machines).

## Sources

Read on 2026-11-12 by research agents; their reports are in
`~/.cache/handoffs/benchmark-research-2026-11-12.md`. Items the
reports marked "not verified" are marked so above.

js-framework-benchmark and 7GUIs:
- https://github.com/krausest/js-framework-benchmark (README; the
  benchmark definitions in `webdriver-ts/src/benchmarksCommon.ts` and
  `benchmarksPlaywright.ts`; the row page in the vanillajs `Main.js`)
- https://github.com/krausest/js-framework-benchmark/wiki/How-the-duration-is-measured
- https://github.com/krausest/js-framework-benchmark/wiki/Computation-of-the-weighted-geometric-mean
- https://eugenkiss.github.io/7guis/tasks

GPUI:
- https://crates.io/crates/gpui (0.2.2) and
  https://docs.rs/crate/gpui/0.2.2/source/Cargo.toml (blade-graphics,
  taffy, cosmic-text, x11rb, wayland-client)
- https://github.com/zed-industries/zed/pull/46758 (blade replaced by
  wgpu on Linux, merged 2026-02-13)
- https://docs.rs/gpui/latest/gpui/ (TestAppContext, test-support)
- Zed's headless and visual test contexts: commit
  8c0b088366545e5050711066a4be666266f28110 and
  `plans/visual-test-improvements.md` in Zed's repository (read
  through a mirror; not verified against github.com)

Blitz:
- https://github.com/DioxusLabs/blitz (README: beta status; crates
  blitz-dom 0.3.0-beta.4, dioxus-native 0.8.0-alpha.3)
- https://github.com/DioxusLabs/anyrender
- https://blitz.is/status/css
- https://github.com/DioxusLabs/blitz/blob/main/wpt/runner/src/main.rs
  (offscreen rendering through `VelloImageRenderer` and
  `VelloCpuImageRenderer`)

iced:
- https://crates.io/crates/iced (0.14.0, 2025-12-07)
- https://docs.rs/iced_test/latest/iced_test/
- https://github.com/iced-rs/iced/releases/tag/0.14.0
- https://discourse.iced.rs/t/virtualized-lists/1190
- https://github.com/dcampbell24/iced_7guis

Slint:
- https://crates.io/crates/slint (1.18.1, 2026-09-21)
- https://docs.rs/i-slint-backend-testing/latest/i_slint_backend_testing/
- https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/guide/backends-and-renderers/backends_and_renderers.mdx
- https://docs.slint.dev/latest/docs/slint/reference/std-widgets/views/listview/
- https://github.com/slint-ui/slint/tree/master/examples/7guis

egui:
- https://crates.io/crates/egui (0.37.0, 2026-10-15)
- https://docs.rs/egui_kittest/latest/egui_kittest/
- https://docs.rs/egui/latest/egui/containers/scroll_area/struct.ScrollArea.html
- https://github.com/rust-ninja-sabi/rust-egui-seven-tasks

vapors:
- `docs/plans/perf-finish-line.md`, `docs/perf/baseline-2026-11-10.md`,
  `docs/perf/mount-2026-11-12.md`, `docs/plans/one-scene.md` (D6,
  frame-to-idle), `crates/perf/src/` (`sampler.rs`, `stats.rs`,
  `instructions.rs`, `gpu_time.rs`, `scaling.rs`, `workloads.rs`).
