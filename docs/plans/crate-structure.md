# Crate structure: where vapors' crate boundaries should be, and how to get there

Design only, for after the performance finish line
(`docs/plans/perf-finish-line.md`). Nothing here changes current work.
Written from `origin/main` at adb3b677 (2026-11-13). Every number is
measured by a script in `docs/plans/crate-structure/` with its output
committed beside it, or marked "to be measured" with the method.

Contents

1. Today's structure, measured
2. Where the real boundaries are
3. Candidate target structures
4. Recommendation
5. Migration plan
6. How instruction counts are checked at each step
7. Decisions for Tom
8. Risks
9. Scripts and outputs

## 1. Today's structure, measured

### 1.1 Size

26 crates and xtask, 198,501 lines of Rust (every `.rs` under
`crates/` and `xtask/`, tests included; `wc -l`, 2026-11-13). The
first crates were created on 2026-10-22, the last on 2026-11-12, so
"the last month" below is the whole history.

| crate | lines | files | what it is |
|---|---|---|---|
| elements | 55,494 | 478 | `Document`: the tree, cascade, layout (Taffy), paint to a display list, kept state between updates, events, focus, controls, the a11y mirror. `src/` is 31,815 lines in 204 top-level files plus `layout/` (46 files, 5,687) and `style/` (27 files, 2,965); `tests/` is 23,601 lines in 172+ files |
| app | 19,878 | 167 | `ElementsApp` (the platform `App` over a `Document`), `ScenePainter` (3,640 lines: kept GPU batches, surfaces), `DocumentInput`, tooltips |
| css | 15,881 | 163 | tokenizer, parser, AST, selectors, cascade, values; depends on scene for `Color` only |
| compiler | 12,028 | 106 | `.wisp` to Rust; syn, quote, prettyplease; a build dependency of every crate with templates |
| text | 10,882 | 91 | parley shaping and layout, skrifa/zeno rasterising, the glyph and icon atlas, the text pipeline; 6 of its 62 `src` files (497 lines) name `wgpu` |
| platform | 9,738 | 101 | winit window, web canvas, input, IME, clipboard, picker, the a11y bridge; the only crate allowed `web_sys` (`[imports.web_sys]`) |
| perf | 6,844 | 74 | the finish-line harness (instruction counts, timed runs, workloads) |
| elements-corpus | 6,743 | 56 | Chromium oracle fixtures and checks |
| renderer | 6,499 | 61 | quad, shadow, solid and image pipelines; `Retained` buffers; GPU timing |
| dom | 4,282 | 41 | the `Dom` trait and blocks generated code calls |
| racer | 3,593 | 38 | the demo game: HUD, 3D craft |
| canvas | 2,863 | 23 | a window or canvas with a painter, for apps without a document |
| scene3d | 2,806 | 27 | 3D nodes, meshes, camera, maths (glam); no GPU |
| signals | 2,796 | 26 | the reactive graph; `std` only |
| scene3d-gpu | 1,930 | 21 | `SceneRenderer` |
| compiler-examples | 1,880 | 26 | templates the compiler's tests build |
| scene | 1,373 | 12 | the 2D display list (`Item`, `Scene`, `Rect`, `Color`), batching, scissors; no dependencies |
| scene3d-components | 1,149 | 14 | `<Mesh>` and friends as components |
| devtools | 1,028 | 8 | the Chromium DevTools client for the oracle |
| components | 732 | 11 | `.wisp` components over dom and signals |
| pixels, demos, gpu, gpu-harness, a11y, web-demo | 141 to 644 | | leaves and shared bits |
| xtask | 27,767 | | the factory |

### 1.2 The crate graph

`docs/plans/crate-structure/deps.py` on `cargo metadata --all-features`
(output: `deps.txt`). Depth is the longest chain of workspace crates
below a crate; "dependents" counts workspace crates whose linked
closure includes it. Build dependencies (the compiler, compiled for
the host) and dev dependencies are not in the columns.

```
depth 1  signals  scene  scene3d  gpu  compiler  devtools  pixels
depth 2  dom(signals)  css(scene)  renderer(scene, wgpu)
depth 3  a11y(dom)  text(renderer, scene)  components(dom, signals)  gpu-harness
depth 4  elements(a11y, css, dom, scene, text)  platform(a11y, gpu, winit)
depth 5  app(elements, platform, renderer, css, dom, scene, text)  canvas  elements-corpus
depth 6  demos  racer  scene3d-gpu(app, scene3d)  scene3d-components(app, scene3d)
depth 7  perf  web-demo
```

Most depended on: signals (15 dependents), scene (14), dom (14),
renderer (12), a11y (11), gpu (10), text (10), css (9), elements (8),
platform (8).

What each crate links, by external package count in its closure:
signals, scene, dom, css 0; scene3d 1; compiler 5; a11y 7; gpu 132;
renderer 134; text 195; **elements 199**; platform 262; app 310;
web-demo 311; perf 316. elements links wgpu, naga and the font stack
because it depends on text, which holds the atlas and pipeline beside
the shaping: `Document` owns a `FontSystem` and a `TextCache`, and 43
of its 277 `src` files name `vapors_text`.

Generated code (the compiler's output) names `vapors_dom` and
`vapors_signals` only (`grep` over `crates/compiler/src` and the
generated examples): the "small shared runtime" the rules require is
those two crates, plus `vapors_components` when a template mounts one.
touche today depends on canvas, gpu, gpu-harness, platform, renderer,
scene and text, not on elements, dom or app
(`~/projects/touche/crates/*/Cargo.toml`).

The layering check (`cargo xtask check-layers`, `xtask/src/layers.rs`,
`layers_graph.rs`) reads `[layers.forbid]` in `.config/factory.toml`
and refuses a forbidden crate anywhere in a crate's normal or build
closure. vapors has one rule: `vapors-components` must not reach
elements, platform, app, renderer, gpu, canvas, text or css (design 03,
"A component's own events": a component reaches the tree only through
`Dom`). The imports check (`[imports.web_sys]`) keeps `web_sys` in
`crates/platform/`.

### 1.3 Build time

A clean `cargo build --workspace --timings` of the dev profile with no
sccache and no incremental, `CARGO_BUILD_JOBS=3`, `nice`, while the
one-minute load was between 20 and 50 (other agents' builds): 334.9 s
wall, 993.9 CPU-seconds summed over units
(`docs/plans/crate-structure/timings.py`, output
`timings-clean-build.txt`). Absolute times are inflated by the load;
the shares are what to read.

The 27 workspace units are 126.6 s, **12.7 % of a clean build**. The
rest is dependencies: syn 62.8 s, naga 42.2, x11rb-protocol 40.4,
wgpu-core 35.7, ash 34.9, rustix 33.7, read-fonts 32.3, image 30.3,
the wayland protocol crates 46.8 between two. Within the workspace:

| crate | s | % of workspace | % of all |
|---|---|---|---|
| text | 23.8 | 18.8 | 2.4 |
| compiler | 19.7 | 15.6 | 2.0 |
| **elements** | 17.5 (front end 10.9, codegen 6.6) | 13.8 | 1.8 |
| css | 13.4 | 10.6 | 1.3 |
| xtask | 10.8 | 8.5 | 1.1 |
| elements-corpus | 9.2 | 7.2 | 0.9 |
| renderer | 5.4 | 4.3 | 0.5 |
| perf | 4.8 | 3.8 | 0.5 |
| racer | 4.1 | 3.2 | 0.4 |
| platform | 3.0 | 2.3 | 0.3 |
| app | 2.0 | 1.6 | 0.2 |
| the other 16 | 10.9 together | 8.6 | 1.1 |

So a clean build is not where crate structure matters: sccache serves
dependencies from the shared cache in every worktree, and the whole
workspace is an eighth of the work. What matters is the edit loop: a
change in crate X recompiles X (its lib, its unit-test binary and its
integration-test binaries) and every unit downstream of X, and
`cargo xtask ci`'s unit tier builds the test targets of the whole
workspace. Measured after the clean build, same conditions:

**The unit tier's build** (`cargo test --workspace --no-run --timings`
after the clean build; load 30 to 60; output
`timings-test-targets.txt`): 816 units, 2,833 s wall, 8,454
CPU-seconds. Two things the clean build did not show:

- `[profile.test] opt-level = 1` (root `Cargo.toml`) is a different
  profile from `dev`, so the test build compiles the whole dependency
  tree again at opt-level 1: naga 314 s, wgpu-core 251 s, syn 250 s,
  winit 171 s, x11rb-protocol 132 s. sccache serves these to other
  worktrees once one has built them; the first unit tier in a fresh
  cache pays them.
- The workspace is 54 % of this build (4,567 s), and **test binaries
  are 67 % of the workspace's share**: 479 units, 3,076 s. elements
  is 1,685.7 s, **19.9 % of the whole unit-tier build and 36.9 % of
  the workspace's part**: its lib at opt-level 1 is 246.8 s and its
  173 test binaries (172 files under `tests/` and the unit-test
  harness) are 1,434.9 s, a mean of 8.3 s and a maximum of 89.9 s
  each. app is 599 s (lib 101.5, 55 test binaries 376.8); compiler 211
  (45 binaries, 2.3 s each); css 183 (60 binaries, 1.7 s each);
  compiler-examples 210 (23 binaries that each build templates).

Each test binary is a separate rustc invocation that compiles its file
and links the crate's closure; the per-binary mean for elements (8.3
s) against css (1.7 s) is the size of that closure. So in the tier
every worktree runs on every push, the cost of elements is four fifths
its integration tests, and a crate split that leaves the 172 binaries
in place changes a fifth of elements' cost.

**After one change in elements** (`touch crates/elements/src/lib.rs`,
css and scene untouched):

- `cargo build --workspace` (libs only, dev profile; output
  `timings-after-elements-touch-build.txt`): 53.0 s wall, 127
  CPU-seconds over 12 units, load 29 to 41: elements 40.7 s (17.5 s
  in the clean build an hour earlier; the difference is load, which
  is the noise on every absolute number here), racer 27.4, demos 18.2,
  perf 17.1, app 8.1, elements-corpus 6.8, web-demo 6.4, scene3d-gpu
  1.4, scene3d-components 0.9.
- `cargo test --workspace --no-run`: the direct run was lost when the
  worktree's target directory was emptied by another process at 08:53
  while it ran (the disk was at 85 %). Derived instead from the
  per-unit test-target report and the graph, summing every crate the
  change invalidates: an elements change rebuilds 10 crates, 3,254 s
  of the workspace crates' 4,416 s (74 %), of which 289 test binaries
  are 2,388 s; a css change 11 crates, 3,437 s (78 %), 349 binaries;
  a scene change 17 crates, 3,926 s (89 %), 389 binaries. The derived
  sum is an upper bound on wall time with 3 jobs and the right order
  of magnitude for CPU; measuring it directly is step 1 of section 5.

**The web page.** Not timed: a wasm build of the page is a second
full dependency build (`cargo build --target wasm32-unknown-unknown
-p vapors-web-demo --timings`, or `cargo xtask web` for the release
page with wasm-opt), and under the day's load it was not practical
beside the native measurements. What is known from the graph: the
page links 16 workspace crates and 311 external packages (`deps.txt`,
`vapors-web-demo`); the compiler is a build dependency of five of
them and is never linked; platform's wasm-only dependencies are
`web-sys`, `js-sys`, `wasm-bindgen`, `wasm-bindgen-futures`,
`web-time` and `console_error_panic_hook`
(`crates/platform/Cargo.toml`), and its native-only ones (`arboard`,
`rfd`, `pollster`, `winit`'s X11 and Wayland backends) are behind
`cfg(not(target_family = "wasm"))`. The last served release page
(`~/srv/vapors-demo`, 2026-10-23) is 7,029,485 bytes of wasm before
compression; the current size is to be measured with `cargo xtask web`
and `ls -l target/web/*.wasm*` in a quiet window. No crate split in
this plan changes what the page links; the peels move code between
crates the page already links.

What recompiles after an elements change, from the graph: elements'
lib, unit tests and 172 integration-test binaries, then app,
elements-corpus, demos, racer, scene3d-gpu, scene3d-components, perf,
web-demo, and compiler-examples' tests (dev-dependency on elements).
Each file under a crate's `tests/` is its own binary linked against
the crate and its whole closure: elements has 172, css 59, app 54,
compiler 43, renderer 18, dom 15, text 9 (`ls crates/*/tests/*.rs`). Only compiler, css, dom, signals, scene, scene3d, text,
renderer, gpu, platform and a11y stay warm.

### 1.4 Churn and co-change

`docs/plans/crate-structure/churn.py` over the 252 commits on `main`
since 2026-10-13 (output `churn-30d.txt`):

- Commits touching each crate: elements 105 (42 %), app 65 (26 %),
  platform 45, elements-corpus 42, xtask 37, text 27, compiler 26,
  renderer 23, canvas 23, perf 22, demos 21, dom 19; css 15, scene 15;
  scene3d, signals, components, a11y under 6 each.
- Crate pairs changed in one commit: app+elements 46, elements+corpus
  36, app+platform 27, app+corpus 24, elements+platform 23,
  demos+elements 17, app+text 16, canvas+platform 16, elements+perf 15,
  elements+text 15, dom+elements 14, css+elements 13, compiler+dom 13,
  compiler+compiler-examples 13, elements+scene 12.
- Inside elements, by the clusters in `clusters.txt` (section 2.3):
  document 77 commits, update 52, paint 49, layout 42, events 38,
  style 31, controls 23, a11y 15, svg 4. Pairs: document+update 45,
  document+paint 43, paint+update 37, document+layout 36,
  layout+update 35, a11y+document 15, controls+events 14.
- The most-changed file is `crates/elements/src/lib.rs`, in 67 of 252
  commits: it declares 161 private modules, so every new file edits it.
  That is the one structural source of merge conflicts between
  parallel branches in elements that a crate split (or `mod` files per
  cluster) removes. Then `update.rs` 38, `paint.rs` 31, `painted.rs`
  27, `layout/mod.rs` 25, `document.rs` 22.

app and elements change together in 46 of app's 65 commits; the
elements-to-painter contract (`Painted`, `KeptBatched`, `FrameDraw`,
`ChangedSlots`) moved with every IU3/IU4 slice. That is the finish
line's work, not a stable pattern, but it says the elements/app line
is the one most often crossed today.

## 2. Where the real boundaries are

### 2.1 By dependency

The lower half of the graph is already well cut: signals, scene, css,
dom, scene3d, compiler and a11y have 0 to 7 external packages each and
clear single responsibilities; their public API is used by many
crates and changes rarely (css 15 commits, scene 15, signals 4). The
layering question is in the upper half, which is one ladder:
text (GPU) → elements → app → everything.

Two edges are heavier than their use:

- **elements → text** pulls wgpu and the atlas into the layout crate
  for `FontSystem`, parley's `Layout`, `LineHeight`, `Icon`/`IconPath`
  and the editor driver. The 6 GPU files in text are 497 lines.
- **text → renderer** (for the pipeline and `Retained` buffers) makes
  text depth 3 and the slowest workspace unit (23.8 s) a dependency of
  everything that lays out text.

### 2.2 By data ownership

`Document` (`crates/elements/src/document.rs`) owns 27 fields. Grouped
by the cluster whose code reads and writes them:

| owner cluster | fields |
|---|---|
| document (tree) | `arena`, `root`, `body`, `declared_events` |
| style | `sheets`, `dirty`, `kept` (restyle), `fonts` (`FontSystem`, also read by layout) |
| layout | `texts` (`TextCache`), `boxes` (`KeptBoxes`) |
| paint | `painted`, `checks` (`KeptChecks`), `images` |
| update | `changes`, `counts` |
| events / focus | `pointer`, `focused`, `settle`, `window`, `space_on`, `shortcut_modifier`, `keyboard`, `scroll`, `thumbs_shown` |
| controls | `committed` (number inputs), `editors` (text fields) |
| a11y | `ax` (`AxMirror`) |

Every field is `pub(crate)` and read across clusters through
`impl Document` blocks: 78 blocks in 73 files, 1,196 `pub(crate)`
items, 681 `self.method()` calls. The crate is one type with its
methods spread over 200 files; the file split is by topic, the type is
not split at all. Rust allows an inherent `impl` only in the type's
own crate, so any split of elements that keeps `Document` as it is
must turn the moved methods into free functions over a borrowed
context, or into extension traits, and the fields they touch into
`pub`.

### 2.3 By module graph

`docs/plans/crate-structure/modgraph.py crates/elements/src --clusters
clusters.txt` (output `modgraph-elements.txt`) groups the 277 `src`
files into nine clusters by file-name prefix and counts, for each pair,
how many modules of A name an item of B through `crate::`, `super::`
or a sibling path. Method calls on `self` are not edges (they do not
name a module), so these counts are a floor.

| cluster | files | lines | names items in |
|---|---|---|---|
| paint (paint*, painted, kept_*, frames*, frame_extents*, clip*, stacking, repaint, changed_slots, images, svg_paint, text_paint, ...) | 80 | 9,101 | layout (46 modules, 11 items), document (22, 7), style (12, 7), update (10, 5), events (5, 3), svg (3) |
| layout (`layout/`, placement*, update_layout, lay_out_text, bidi, inline_check) | 56 | 6,826 | paint (5, 8), document (6, 8), controls (1, 8), style (2, 3), svg (1, 3), events (4, 2), update (3, 2) |
| style (`style/`, restyle*, style_checks, media_checks, focus_style, run_bits) | 41 | 4,375 | update (14 items), document (10), paint (4), layout (3), controls (1), svg (1) |
| controls (text_*, number*, range*, checkbox) | 28 | 3,248 | events (10 modules, 7 items), document (20, 3), update, style |
| a11y (a11y_*) | 26 | 3,196 | document (23 modules, 3 items), layout (6, 4), controls (3), paint (3), events (2) |
| events (events*, focus*, scroll*, keys, shortcuts, typing, tooltip_text) | 19 | 2,167 | controls (10 modules, **24 items**), document (19, 8), layout, style, paint |
| document (document, arena, node*, tree, slab, chain, dom_impl, tags, html, css_element, error, lib) | 15 | 1,631 | every other cluster (4 to 6 items each) |
| update (update*, check*, test_rows) | 6 | 694 | document, paint, layout, events, style |
| svg (svg*) | 6 | 577 | document, paint, style, update |

Read as a graph: **document** is the hub (every cluster uses `Arena`,
`Document`, `NodeMap`, `node::Kind`); **paint → layout** is the
heaviest directed edge (46 modules read `LayoutBox`, `BoxRect`,
`Content`, `InlineRun`, `Placement`, `TextCache`); **layout → paint**
exists too (8 items: `ChangedSlots`, `Painted`, `PaintedFrom`,
`Stackings`, `paint::snapped`, `svg_paint::icon`, `images::Images`),
so layout and paint are a cycle today, not a layer pair; **style →
update** is a cycle through `check::*` and `test_rows`; **events ↔
controls** is the densest two-way pair (24 items one way, 7 the
other). a11y and svg are the two clusters with one-way edges only.

The cycles are the real finding. A crate graph is acyclic, so the
order of work in any split is: break the cycles inside the crate
first, with the boundary check that section 5 adds, and only then
move files. The cycles today are:

1. layout → paint: `ChangedSlots` is written by layout
   (`layout/keep.rs`, `placement_changes.rs`, `update_layout.rs`) for
   paint to read, and `update_layout.rs` reads `Painted`,
   `PaintedFrom` and `Stackings`; `paint::snapped` is called from
   `placement_check_page.rs` and `svg_paint::icon` and
   `images::Images` from `layout/boxes.rs`. Resolution: the shared
   records (`ChangedSlots`, the kept-paint types, `Stackings`) move to
   a records module both depend on; the snapping helper to scene (it
   is device-pixel arithmetic); the icon and image natural-size
   queries behind a trait layout owns.
2. style → update: `check::describe`, `check::elements`,
   `check::is_type` and `test_rows` are refusal and test helpers.
   Resolution: `check*` is tree-level (it names elements), so it is
   document; `test_rows` is a test fixture and goes to the test
   support module.
3. events ↔ controls: pointer and key dispatch calls into each
   control's handlers (`number_scrub::pressed`, `range_drag::moved`,
   `text_input::key`, 24 items), and controls call `events::capture`,
   `events::tree`, `keys::types_text`. Resolution: controls are
   handlers registered with dispatch (a table from `node::Kind` to a
   handler set), the shape design 03 already gives components; then
   events does not name any control.

### 2.4 By rate of change and who edits together

The lower crates are stable (section 1.4). Inside elements the
clusters that change together are document+update+paint+layout, which
is the update pipeline: a change to what an update keeps touches the
records (`document`), the driver (`update`), and two of the three
phases. That is the finish line's work and will slow once the
properties hold. a11y, svg and controls change apart from the
pipeline and from each other; events and controls change together.

So by every measure the same picture: a stable, well-cut lower half;
one 32 k-line crate whose internal clusters are real (a11y, svg,
controls and events have few, one-way, nameable edges) but whose
pipeline core (document, update, style, layout, paint) is one type
with cycles between its phases; and an app crate that changes with
elements on every paint-contract slice.

### 2.5 What a split costs in public API

Peeling a cluster into its own crate makes every item it reaches in the
core public. From `modgraph-elements.txt`, counting named imports
(section 2.3) plus methods called on `self` whose definition is in
another cluster (the second pass of `modgraph.py`; a name defined in
two clusters is "ambiguous" and counted once):

| cluster peeled | core items it names | core methods it calls on `self` | ambiguous tree accessors | items the core names in it | total new `pub` |
|---|---|---|---|---|---|
| a11y | 15 (`Document`, `NodeMap`, `NodeSet`, `BoxRect`, `Key`, `Slots`, `NO_SLOT`, `ChangedSlots`, `padding_box`, `Painted`, `Modifiers`, `scroll::applied`, 3 control setters) | 11 (`active_element`, `focus_candidate`, `is_disabled_control`, `scroll_node_by`, `attribute`, `contains`, `root`, `is_text_input`, `number_field`, `note_change`, `update`) | 9 (`checked`, `children`, `elements`, `hit`, `parent`, `range`, `tag`, `text`, `value`) | 5 (`AxError`, `AxTakeCounts`, `AxTaker`, `AxMirror`, `note_replaced`) | about 40 |
| controls | 14 named (`Arena`, `Document`, `Element`, `Asked`, `Point`, `capture`, `tree`, `Modifiers`, `types_text`, `selected_all`, `TextStyle`, `is_type`, ...) | 6 | 0 | **38** (24 from events, 8 from layout, 4 from document, 1 from style, plus `editor_from_text`, `number_focused`, `text_focused`, `focused_text_marks`) and two `Document` fields (`committed`, `editors`) | about 60, two of them fields |
| svg | 7 (`Arena`, `ElementError`, `Kind`, `NodeMap`, `relative`, `Used`, `describe`) | 0 | 0 | 7 (`is`, `layout_style`, `natural`, `positive`, `view_box`, `svg_checks::refusals`, `svg_paint::icon`) | about 14 |
| paint (from a core of document, style, layout) | 11 layout + 7 document + 7 style + 5 update + 3 events + 3 svg | 3 | 0 | 8 from layout, 6 from document, 4 from style, 3 from update, 2 from a11y | about 60, and `Document`'s `painted`, `checks`, `images` fields |
| style, layout, paint each their own | the union of the rows above and the phase records | | | | over 150, and most of `Document`'s 27 fields |

The ambiguous names are the tree accessors (`children`, `parent`,
`tag`, `text`, `value`, `checked`) that exist as both `Document`
methods and `Arena`/`Element` methods; a peel makes the `Document` ones
public, which is a tree API vapors wants anyway (invariant 4,
`ARCHITECTURE.md`: one element tree, `Document` implements `Dom`).

## 3. Candidate target structures

Three candidates. All three keep the lower half (signals, scene, css,
dom, scene3d, compiler, a11y, renderer, gpu, platform) as it is: the
measurements give no reason to touch it. They differ in what happens
to elements, app and text.

### A. Phase crates

Split elements by pipeline phase, one crate each:

```
vapors-tree      arena, node, node_map, slab, chain, tree, tags, html,
                 css_element, dom_impl, document (the struct), error,
                 update (the driver), check*                 ~2.3k lines
vapors-style     style/, restyle*, style_checks, media_checks,
                 focus_style, run_bits                       ~4.4k
vapors-layout    layout/, placement*, update_layout, lay_out_text,
                 bidi, inline_check                          ~6.8k
vapors-paint     paint*, painted, kept_*, frames*, frame_extents*,
                 clip*, stacking, changed_slots, images, ...  ~9.1k
vapors-input     events*, focus*, scroll*, keys, shortcuts, typing ~2.2k
vapors-controls  text_*, number*, range*, checkbox           ~3.2k
vapors-ax        a11y_*                                      ~3.2k
vapors-svg       svg*, svg_paint                             ~0.9k
```

Allowed dependencies: tree ← style ← layout ← paint, input ← tree
and layout, controls ← input and layout, ax ← all, svg ← tree and
style; app depends on paint, input, controls, ax. `Document` stays in
tree with every field `pub`; the 78 `impl Document` blocks outside
tree become free functions or extension traits.

- Build time: the best parallelism cargo can get from elements (eight
  units in place of one), and an edit in paint recompiles paint, ax,
  controls and app, not style and layout. The front end of elements
  is 10.9 s of its 17.5 s; eight crates' front ends overlap, so the
  wall time of an elements-wide change falls, but the sum rises
  (every crate re-reads the lower crates' metadata).
- Runtime: the update is `style`, then `layout`, then `paint` on one
  `&mut self` (`update.rs:59-86`); per update the phase calls are a
  handful, but per box the paint phase reads layout's records through
  accessors (`BoxRect`, `Placement`, `TextCache`, `Content`) and
  layout reads style's `Used`. Each accessor becomes a cross-crate
  call. Section 6 says what makes it inlinable and how it is checked;
  the risk is real and per-element.
- API surface: over 150 new `pub` items and most of `Document`'s
  fields (section 2.5). Everything the finish line's slices change
  (`ChangedSlots`, `KeptBoxes`, `Painted`, `Stackings`) becomes a
  cross-crate contract that must be kept coherent in three crates per
  slice.
- Layering clarity: highest, once the three cycles are broken; the
  `layers` step can state the ladder.
- Agent parallelism: a branch in paint and a branch in layout no
  longer share `lib.rs`; but co-change says they share the slices
  (paint+layout 35 of 49 paint commits), so they would share three
  crates' worth of PRs instead.
- Migration: the largest. Three cycles, one struct with 27 fields,
  1,196 `pub(crate)` items to classify.

### B. Peel the consumers

Keep the pipeline core as one crate and move out the clusters whose
edges are one-way, in the order their API cost allows:

```
vapors-elements      document, update, style, layout, paint, events,
                     svg, controls (for now)                ~26k lines
vapors-elements-ax   a11y_*: the mirror, projection, driver, actions
                                                             ~3.2k
vapors-controls      text_*, number*, range*, checkbox, when the
                     handler table exists (section 5, step 6) ~3.2k
vapors-text-gpu      the 6 wgpu files of text (atlas texture, pipeline,
                     draw page), so text and elements link no wgpu
                                                             ~0.5k
```

Allowed dependencies: elements-ax ← elements, a11y, dom, serde;
controls ← elements, text; text-gpu ← text, renderer; elements ← text
(CPU), css, dom, scene, a11y; app ← elements, elements-ax, controls,
text-gpu, renderer, platform. The ownership change that makes the
first peel clean: the `AxMirror` moves out of `Document` (field `ax`)
to the caller that owns the painter today (`ElementsApp`), fed from
the update's `Changes` the way `Painted` is. `Document` keeps no
knowledge of the mirror; `note_replaced` becomes an entry in
`Changes`.

- Build time: an a11y change no longer recompiles the core or its 172
  test binaries; a core change still recompiles ax, controls, app.
  The core shrinks by about 20 %, the test binaries that use a11y
  (the `a11y_*` and `apg_*` tests) move with it.
- Runtime: the a11y phase runs once per frame after the update
  (phase 5 of the scheduler) and reads the tree through the accessors
  above; no per-box work in the update crosses a crate. Controls'
  handlers are called per event, not per update. No new boundary in
  the style → layout → paint path. text-gpu adds a boundary at the
  atlas upload, once per frame per changed page.
- API surface: about 40 items for ax, about 14 for svg if it goes, 60
  for controls (so controls wait for the handler table that removes
  the 24 events → controls names). Fields: none for ax once the
  mirror is owned outside; `committed` and `editors` for controls.
- Layering clarity: the `layers` step gains three rules (ax, controls
  and text-gpu are not reachable from the core; the core is not
  reachable from css, dom, text).
- Agent parallelism: a11y and controls branches stop touching the
  core's `lib.rs` and tests; the core still has one `lib.rs`, so C's
  directory regrouping is part of B.
- Migration: three moves, each one PR, each behind a module-boundary
  check that proves the edge is one-way before the move.

### C. One crate, grouped

No new crates. Inside elements, each cluster becomes a directory with
its own `mod.rs` (as `layout/` and `style/` already are): `paint/`,
`a11y/`, `controls/`, `events/`, `document/`, `update/`, `svg/`.
`lib.rs` declares nine modules, not 161. The 172 integration-test
files become one binary (`tests/main.rs` with `mod` per file), or one
per cluster. The three cycles are broken as in A and B, and a
module-boundary check (section 5, step 2) keeps them broken.

- Build time: no change to the lib's compile. The test binaries: 172
  links of elements and its 199-package closure become one or nine.
  Today they are 1,434.9 s of elements' 1,685.7 s in the unit tier
  (section 1.3), a mean of 8.3 s per binary against 1.7 s for css's;
  cargo's report does not separate the link step from compiling the
  test file, so the share that one binary saves is step 1's
  measurement. Whether sccache serves a test binary from another
  worktree's build is to be checked in the same step (`sccache
  --show-stats` before and after a second worktree's unit tier).
- Runtime: none; the crate is one compilation unit as today.
- API surface: none.
- Layering clarity: the module check gives the same ladder as A
  inside the crate, readable in `clusters.txt` and refused on
  violation; it is not visible to `cargo` or the `layers` step.
- Agent parallelism: the `lib.rs` conflict goes; a new file in paint
  edits `paint/mod.rs`. Co-change inside the pipeline stays what the
  work makes it.
- Migration: a `git mv` script and a `mod` rewrite, one PR per
  cluster, no semantic change, instruction counts identical by
  construction (same crate, same code).

## 4. Recommendation

C first, then B's a11y peel, then B's text-gpu split if its measured
closure gain is worth a crate, and controls only when the handler
table exists. A is not recommended: its benefits (parallel compile of
the pipeline, a cargo-visible ladder) are bought with a boundary
inside the per-box hot path and a contract across three crates for
every finish-line slice, and the co-change data says the pipeline's
phases are edited together.

Why C before anything: it is the prerequisite of every peel (the
clusters must be explicit and the cycles broken before a crate line
can be drawn), it has no runtime or API cost, and it removes the one
measured conflict hotspot (`lib.rs` in 67 of 252 commits). If the
test-binary measurement shows what 172 separate links suggest, the
test consolidation is the largest build-time win available in this
plan and it needs no crate change at all.

Why the a11y peel: it is the one cluster whose edges are one-way with a
small, nameable surface (about 40 items), whose state (`AxMirror`)
belongs outside `Document` by the architecture's own ownership rule
(the mirror is a consumer of updates, like the painter), and whose
work (APG patterns, the driver, the web mirror) runs in parallel with
pipeline work on different branches. Its peel also moves the serde
dependency (the driver's `Command` and `Reply`) out of the core.

Why not app into elements, despite 46 co-changes: app depends on
platform (winit, web-sys); merging would put the window stack under
the layout engine, and touche's non-UI crates would link it. The
co-change is the paint contract moving, which ends with the finish
line.

The target graph (B after C), with the future areas placed:

```
leaves     signals   scene   scene3d   compiler   pixels   devtools
                              └ particles (planned, no GPU)
                              └ shapes (planned, no GPU, no text; parser
                                moved from text; compiler depends on it)
depth 2    dom(signals)   css(scene)   renderer(scene, wgpu)   pool (planned)
depth 3    a11y(dom)   text(scene; CPU only after text-gpu)   components(dom, signals)
           services (planned, web-apis-native D1 b)
depth 4    elements(a11y, css, dom, scene, text, shapes, pool)
           platform(a11y, gpu, winit, services, pool)   text-gpu(text, renderer)
depth 5    elements-ax(elements)   controls(elements, text)
           app(elements, elements-ax, controls, text-gpu, platform, renderer)
           canvas   elements-corpus
depth 6    demos   racer   scene3d-gpu(app, scene3d)   scene3d-components
           gltf, scene3d-anim, audio, scene3d-bevy (planned, G1: each
           linked only by an app that uses it)
           ios, android (planned platform crates)
depth 7    perf   web-demo
```

Where each future design lives, from its own document:

| design | lives in | new crate | dependency it adds |
|---|---|---|---|
| flat-scene (`~/.cache/w-design-flat/vapors/docs/plans/flat-scene.md`) | scene (primitive record, order key, `TransformClass`), renderer (the one pass: cull, sort, batch), elements paint (emits primitives), compiler (`PaintUses` per crate) | none named | compiler emits a table the runtime reads; no new crate edge |
| svg-support, one-scene (1.1) | `vapors-shapes`: path parser moved from text's `icon_path.rs`, classifier, tessellator, table format, process registry | `vapors-shapes` | compiler → shapes; elements → shapes; renderer → shapes; "no GPU and no text dependency" |
| 3d-engine-parity (G1, E7) | scene3d-gpu features for pipeline variants; separate crates for anything with a dependency or backend | `vapors-gltf`, `vapors-scene3d-anim`, `vapors-audio` | each linked only by an app that uses it |
| particles (P7) | beside scene3d; used by scene3d-gpu's tests and debug check | `vapors-particles` (no GPU) | scene3d-gpu → particles |
| threading | `crates/pool`, used by platform (owns `Waker`) and elements; web implementation in platform | `vapors-pool` | platform → pool, elements → pool |
| platforms | platform stays the trait home; iOS and Android bridges in their own crates | `vapors-ios`, `vapors-android` | platform ← ios/android (or app) |
| web-apis-native (D1) | (a) in platform, or (b) a services crate platform depends on, so touche's runtime uses services without winit | `vapors-services` (option b) | platform → services |
| ecs (D) | optional glue | `vapors-scene3d-bevy` | app → scene3d-bevy if wanted |
| async-components (3.4) | signals (`task.rs`, `executor.rs`, `deadline.rs`), dom (`blocks/awaiting.rs`), compiler | none | none |
| build-time-styles (B0) | compiler reads sheets; `scope.rs` and the UA sheet move into css | none | **compiler → css** |
| splice-layout, style-allocations, text-measure | elements (layout lane, style records), text, perf | none | none |
| css-for-3d | css (properties), scene3d-gpu (materials), elements' scene node | none | none |

Two of these change the lower half's graph: compiler → css (B0) and
compiler → shapes. Both are build-time dependencies of every crate
with templates, so css and shapes compile for the host as well as the
target; css is 13.4 s. That is a reason to keep css free of anything
heavy, and it already is (0 external packages).

## 5. Migration plan

Every move is a script in `xtask` (`cargo xtask move-modules
<cluster>` reading `clusters.txt`, or a one-off under
`scripts/`), committed with the plan, run once per step, never done by
hand. Each step is one PR under 400 lines of hand-written change (the
moves themselves are mechanical and reviewed by their script), lands
on a green main, and is done in a window with no other elements branch
open, because every step rewrites paths that every open branch uses.
The order:

1. **Measure the test binaries.** `cargo test -p vapors-elements
   --no-run --timings` before and after a `tests/main.rs` that
   declares the 172 files as modules (the `support/` directory becomes
   `mod support`). If the link time is what section 1.3 predicts, land
   it; the same for app (54), css (59) and compiler (43). No source
   change; nextest's per-test timeouts still apply per test.
2. **The module-boundary check.** `cargo xtask check-modules`: reads
   `clusters.txt` (moved to `.config/`), builds the cluster graph the
   way `modgraph.py` does, and refuses an edge the file's `allowed`
   table does not list, naming the file, the line and the item.
   Proven by adding a forbidden `use` and watching it fail. Lands with
   today's edges all allowed, so it is green and only stops new ones.
3. **Break the three cycles**, one PR each, in this order: style →
   update (move `check*` to document, `test_rows` to test support);
   layout → paint (the records module; `snapped` to scene); events ↔
   controls (the handler table: `node::Kind` to a handler set
   registered by controls, dispatch calls through it). Each PR removes
   its edges from the `allowed` table so the check holds them off.
   Instruction counts before and after each (section 6).
4. **Directories.** `git mv` each cluster into its directory with a
   `mod.rs`; `lib.rs` declares nine modules; the 55 `#[path =
   "..._tests.rs"]` test mounts follow their files. One PR per
   cluster, largest first (paint). No code changes; the diff is
   renames and `mod` lines; `cargo xtask ci` green; instruction counts
   identical.
5. **The a11y peel.** First the ownership change: `AxMirror` out of
   `Document` to `ElementsApp` (and to the tests that drive it), fed
   from `Changes`; `note_replaced` becomes a change record. Then the
   crate: `crates/elements-ax`, the a11y tests with it, the `layers`
   rule that the core cannot reach it. The 40 items go `pub` with doc
   comments (`missing_docs` is a warning, warnings are errors).
6. **text-gpu**, if decision D3 says so: the 6 files to
   `crates/text-gpu`; text drops wgpu and renderer; elements' closure
   measured before and after with `deps.py`.
7. **controls**, after step 3's handler table has held for a while
   and when a second control family (design 03's components) needs the
   same registration: `crates/controls`.

Steps 1, 2 and 4 can run before the finish line is reached if a
window with no open elements branch exists, since they change no
behaviour; the plan says after, because such a window does not exist
while the finish-line slices are open.

## 6. How instruction counts are checked at each step

The finish line is judged by instruction counts (`vapors-perf
--scaling`, `crates/perf/src/instructions.rs`: Linux perf events,
user-space instructions of the calling thread, so the count moves
with the code and not with the machine's load) and by the timed run
(`vapors-perf 200 20`, medians of three, load under 1.5,
`docs/perf/timed-2026-11-12.md`). A move that changes no code must
leave the instruction counts the same to the instruction; a move that
adds a crate boundary may change them where a call stops being
inlined.

Per step, before and after, on the step's branch and its base:

```
cargo build --release -p vapors-perf
for change in Idle Text FlexText Class HoveredClass Insert; do
  ./target/release/vapors-perf 60 10 --scaling $change
done
```

and the per-phase counts (`style`, `layout`, `paint`, `draw`,
`update`) at 1,000, 3,000 and 10,000 rows are diffed. Acceptance: for
steps 1, 2 and 4 the counts are identical (any difference is a bug in
the move); for steps 3, 5, 6 and 7 a phase may change by less than
the scaling judge's own floor (2 % and 5,000 instructions,
`perf-finish-line.md`), and a larger rise names the function that
stopped inlining, found with `perf record` on the workload, and is
fixed by `#[inline]` on that function before the step merges. The
timed run is repeated for the step that merges last, in a quiet
window, and recorded in `docs/perf/`.

What governs cross-crate inlining in vapors' builds, so the check is
read correctly:

- vapors has no `[profile.release]`: release builds use cargo's
  defaults, `codegen-units = 16` and `lto = false` (LTO across the
  codegen units of one crate only, not across crates). The web page
  is trunk's `--release` with the same profile (`xtask/src/web.rs`).
  No crate uses `#[inline]` today (0 occurrences in scene, css,
  elements, text, renderer, app, dom, signals).
- A function defined in crate A is inlined into crate B only when B's
  compile can see its body: generic functions, functions marked
  `#[inline]`, and small non-generic functions that rustc marks
  cross-crate-inlinable on its own in optimised builds (rustc's
  automatic cross-crate inlining, rust-lang/rust#116505, in the
  toolchain `rust-toolchain.toml` pins). The exact size heuristic is
  the compiler's and changes between versions; this plan does not
  rely on it, it measures.
- `lto = "thin"` or `"fat"` in `[profile.release]` would let LLVM
  inline across crates at link time regardless of the above, at a
  link-time cost and for the perf binary and the page alike. It is a
  decision (D4), not a default of this plan, because it also hides a
  boundary's cost from the instruction count that is supposed to show
  it.
- Field access on a `pub` field is not a call and costs nothing
  across a crate line; the accessors that matter are methods.

The peels in B add no boundary to the per-box path: the a11y phase
reads the tree after the update, controls are per event, text-gpu is
per frame. A's phase boundaries do, which is why A is measured against
in section 3 and not recommended.

## 7. Decisions for Tom

**Decided 2026-11-13 (Tom): D1 to D6 all as recommended.** D1 (a) folders (C), then peel a11y (B); D2 (a) a check-modules step; D3 (b) text stays one crate; D4 (a) LTO off during the migration, thin LTO measured after; D5 (a) one integration-test binary per crate; D6 (a) after this iteration.

**D1. The target.** (a) C then B as section 4 recommends. (b) A, the
phase crates, accepting the measurement burden. (c) C only, no new
crates. Recommendation: (a).

**D2. The module-boundary check as a factory step.** (a) A new
`check-modules` step in `lint` and `ci`, reading `.config/clusters`
tables per crate, refusing new edges; the same mechanism could later
name clusters in app, text and platform. (b) No check; rely on
review. Recommendation: (a): a boundary nobody checks is a boundary
until the next slice.

**D3. text-gpu.** (a) Split the 6 wgpu files out of text so text and
elements link no GPU (elements' closure measured before and after).
(b) Leave text as it is; the page links wgpu anyway and sccache serves
it. Recommendation: (b) until a consumer exists that wants layout
without a GPU (touche's runtime measuring without a window would be
one; today it does not depend on elements).

**D4. LTO in release.** (a) Keep `lto = false`; boundaries show in
the counts. (b) `lto = "thin"` for the perf binary and the page,
measured once for wall time, link time and page size, before any
peel. Recommendation: (a) during the migration; measure (b) after it
as its own perf slice, since it may be a win independent of crate
structure.

**D5. Test binaries.** (a) One integration-test binary per crate
(`tests/main.rs`). (b) One per cluster. (c) As today. Recommendation:
(a) if step 1's measurement shows the link time; (b) if a per-cluster
binary keeps `cargo test -p vapors-elements --test paint` useful to
agents.

**D6. When.** (a) After the finish line, as the brief says. (b) Steps
1, 2 and 4 in the first quiet window, since they change no behaviour
and remove the `lib.rs` conflict that the remaining finish-line
branches pay for now. Recommendation: (a) unless a window with no
open elements branch appears first.

## 8. Risks

- **The build measurements were taken under load 20 to 50.** Shares
  are robust to that, wall times are not; step 1 repeats its
  measurement in a quiet window before deciding.
- **The module graph undercounts.** `self.method()` calls were
  matched by name; names defined in two clusters were counted once,
  trait methods and closures not at all. The check in step 2 is built
  on the compiler's view (a `syn` walk with resolved `use` paths, or
  rust-analyzer's), not on this script, and its first run gives the
  real counts.
- **A peel exposes an API that the finish line is still changing.**
  `ChangedSlots`, `Painted` and `KeptBoxes` are named by the a11y
  cluster; if the peel lands while those move, every slice edits two
  crates. The plan's ordering (after the line) is the mitigation; if
  D6 (b) is chosen, step 5 still waits.
- **Instruction counts that move for the wrong reason.** A moved
  function can change its codegen-unit assignment and its inlining
  within the crate even with no boundary added, since
  `codegen-units = 16` partitions by module. Step 4's "identical"
  acceptance may need `codegen-units = 1` for the comparison build
  only, said in the step.
- **The window with no open branches may not come.** Each step
  rewrites paths every open elements branch uses; rebasing a branch
  across a step is a mechanical path rewrite that the move script
  should also offer (`cargo xtask move-modules --rebase <branch>`),
  so a step does not wait on an empty queue forever.
- **Future crates multiply the graph.** The designs name eleven new
  crates (shapes, particles, pool, services, gltf, scene3d-anim,
  audio, scene3d-bevy, ios, android, text-gpu). Each is justified in
  its document by a dependency an app may not want; none sits on the
  update path. The `layers` and `check-builds` steps are what keep
  them out of the page; `[server_only.builds]` already names the page
  and the app as the builds to check.

## 9. Scripts and outputs

All under `docs/plans/crate-structure/`, run from the repository root
with `python3 -I`; each output file names the report or revision it
was made from in its first lines or in this list.

- `timings.py <cargo-timing.html>`: per-package build time split into
  lib, test binaries (count, mean, max) and build scripts, the
  heaviest external packages, and the unblock chain ending at the last
  unit (with `jobs=3` the queue, not the graph, decides the wall time,
  so the chain is short). Outputs: `timings-clean-build.txt` (dev
  profile, `cargo build --workspace`), `timings-test-targets.txt`
  (`cargo test --workspace --no-run` after it),
  `timings-after-elements-touch-build.txt` (`cargo build --workspace`
  after `touch crates/elements/src/lib.rs`). All at adb3b677,
  `CARGO_BUILD_JOBS=3`, `CARGO_INCREMENTAL=0`, no sccache, `nice`,
  one-minute load 20 to 60.
- `deps.py <cargo metadata json>`: the workspace graph (linked edges;
  build and dev edges apart), depth, dependents, what reaches wgpu,
  winit, web-sys, taffy and serde, and external package counts.
  Output: `deps.txt`.
- `modgraph.py crates/elements/src --clusters clusters.txt`: the
  module graph inside elements by cluster, the items each cross-cluster
  edge names, and the methods called on `self` across clusters by
  definition site. Output: `modgraph-elements.txt`.
- `churn.py . 2026-10-13 --clusters clusters.txt`: commits per crate
  and per elements cluster, co-change pairs, most-changed files.
  Output: `churn-30d.txt` (252 commits).
- `clusters.txt`: the cluster definitions (file-name prefixes, first
  match wins), shared by `modgraph.py` and `churn.py`, and the
  starting point for step 2's `.config` table.
