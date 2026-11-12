# One scene: HTML, SVG and 3D (finish-line property 9)

Property 9 of `docs/plans/perf-finish-line.md`: the racer (3D craft, SVG
markings on it, HTML on top) drawn as one scene with CSS reaching scene
nodes, and no frame over 16.7 ms across a 60 s run. Order of work item
8: the shape table and the one-scene proof, after 3D slice 2's PRs 4,
4b and 5 (`docs/plans/3d-2.md`).

Read from the code at 7d1fe84. Plan only; nothing here is built. Line
counts per PR are estimates. Every number is cited or marked "to be
measured".

## 0. Where things stand

- **Icons** (`docs/icons.md`): `<svg>` with stroked `<path>`s (M L H V A
  Z, `currentColor`, stroke only) is parsed at run time by
  `vapors_text::IconPath`, stroked on the CPU by zeno into an 8-bit mask
  per path, cached in the glyph atlas at device size, and drawn as one
  tinted quad per path (`Item::Icon { icon, rect, colour }`,
  `crates/scene/src/display.rs:137`; built in
  `crates/elements/src/svg_paint.rs`). Capped at 256 device px. The
  compiler does not look at path data; `svg` and `path` are only names
  in its vocabulary (`crates/compiler/src/template/vocabulary.rs`).
- **Build-time style analysis** (Analysis PRs 1 and 2a to 2d merged, 2e
  open as #240): the compiler emits one `static StyleKey` per template
  element (site, scope, tag, literal classes, `class:` toggles, literal
  `style`; `crates/compiler/src/codegen/style_key.rs`), and the tree
  shares one computed style between elements whose share key matches
  (template key, parent's style, states, toggles, selected attributes;
  `crates/elements/src/restyle_share.rs:113`). At 3000 rows the first
  update computes 4 styles plus 2 unkeyed, with 6 live (#240).
- **The racer** (`crates/racer`): its own `Track` renderer on the HUD's
  `gpu-surface` (`crates/racer/src/gpu.rs`). The ground is a full-screen
  triangle whose fragment shader computes road, kerbs, markings and
  shadows per pixel (`track.wgsl`, `fs_main`). Scenery and car are one
  vertex buffer of position, colour and a car flag, with face shading
  baked into the colours (`gpu/mesh.rs`). The camera is a tilted
  orthographic view written by hand in the shader (`vs_solid`: no
  perspective divide, depth `0.5 - (...)/4000`). Simulation coordinates
  are X, Y on the ground and Z up.
- **3D slice 2** (`docs/plans/3d-2.md`): PRs 1 to 3b merged.
  `vapors-scene3d` holds `Scene` (generational node ids, `Group`,
  `Mesh { geometry, colour }`, `Camera`), +Y up, right-handed.
  `vapors-scene3d-gpu`'s `SceneRenderer` draws every mesh node unlit in
  one colour (`shader.wgsl`), one uniform slot per mesh, written only
  when its world version changes. PRs 4 (components), 4b (one cascade
  into the scene) and 5 (lifecycle) are not started.
- **Racer frame cost today** (`docs/perf/baseline-2026-11-10.md`, runs 1
  and 2, 200 frames each, UHD 630): racing, CPU frame median 2.98 to
  3.02 ms, max 4.00 and 4.76 ms; GPU span median 8.16 to 8.57 ms, max
  9.81 and 10.10 ms. The span is an upper bound (it includes the GPU's
  idle gaps between the frame's submits, `crates/perf/src/gpu_time.rs`).

## 1. The shape table

The brief's direction (`docs/briefs/NATIVE_SVG_BRIEF.md`, "Direction:
one shape model"): every painted thing is a shape with coverage computed
at draw time, after transforms; the compiler classifies, deduplicates
and packs literal geometry; at run time an item is a shape id with a
transform, paint and clip. This section fixes what that means for the
first version, which property 9 needs.

### 1.1 One parser, one crate

A new crate, `vapors-shapes`, with no GPU and no text dependency, holds:

- the path-data parser, moved from `vapors-text` (`icon_path.rs`) so the
  compiler, the elements crate and the painter answer "is this path
  data valid" with one function. The compiler gains this dependency;
  today it depends only on syn, quote, proc-macro2 and prettyplease.
- the classifier and the tessellator (1.2);
- the table format: `ShapeTable`, `ShapeId`, `ShapeEntry` (1.3).

The icon route keeps working on the moved parser. Whether icons move
onto shapes is the brief's open question and is not on this plan's path.

### 1.2 What the compiler classifies

For each `<svg>` subtree in a template, each shape element (`<path>`,
`<rect>`, `<circle>`) with literal geometry is classified to the
cheapest exact primitive:

| Primitive | From | Drawn as |
|---|---|---|
| Rect, rounded rect | `<rect>`; a path that is exactly an axis-aligned rectangle | the closed-form distance the box shader already uses (`crates/renderer/src/quads.wgsl`) |
| Circle, ellipse | `<circle>`; a path of two arcs forming one | a closed-form distance |
| Fill mesh | any other closed path with `fill` other than `none` | triangles (lyon), nonzero rule first |
| Stroke mesh | any path with `stroke` | triangles from lyon's stroker, SVG's default miter limit 4 and butt caps, as the icon subset |

Curves are flattened at build time at a tolerance chosen for the largest
scale a shape is drawn at (the brief, "Compile time"); that scale, for
markings seen up close in 3D, is to be measured in S2 against the
antialiasing test.

A literal outside the subset is refused at build time naming file, line
and the fix, as the compiler refuses other attributes today
(`codegen/attribute_refusals.rs`).

### 1.3 Deduplicated and packed

- Two shapes are the same entry when their canonical form is equal:
  absolute commands in viewBox units after parsing, plus the fill rule
  and stroke parameters that change the mesh (width, join, cap,
  miter limit). Paint is not part of the key; it is per item.
- Each crate's build emits one `static SHAPES: ShapeTable` beside its
  `wisp.rs`: vertex and index buffers for every mesh entry, parameters
  for every closed-form entry, and a content hash per entry. Each shape
  element's emitted code names its `ShapeId` within that table.
- The compiler runs per crate from `build.rs` (`build/mod.rs`,
  `compile_dir`), so it cannot see another crate's shapes. A runtime
  registry takes each crate's table at first use and merges entries by
  content hash, so the same marking in two crates is one GPU upload.
  This is decision D3.

### 1.4 What an item is at run time

`Item::Shape` in the 2D display list:

- `shape`: a `ShapeRef`, either a table entry (`ShapeId` after
  registration) or a run-time entry (1.5);
- `transform`: viewBox to device pixels, 2 x 3 affine;
- `paint`: the fill or stroke colour from the element's computed style;
- the clip, from the `Scene`'s clip as for every item.

Shapes join the painter's batching like quads: one pipeline per kind
(closed-form, mesh), instanced, in paint order. On IU4's retained path
(`docs/plans/iu4.md`, PR 2) a shape's instance bytes are kept between
frames like a quad's, so S3 lands after IU4 PR 2.

In a 3D scene the same entry is drawn by `SceneRenderer` with the
world matrix of the node it is attached to (section 2.3), so 2D and 3D
read one table and one GPU upload per entry.

### 1.5 Constant and known-variable properties

The brief's rule (Tom, 2026-11-10): for each painted thing and each
property, constant or known variable; nothing is refused for being
run-time, only tracked. For shapes:

| Property | Constant when | Known variable when | What a change costs at run time |
|---|---|---|---|
| Geometry (`d`, `x`, `y`, `width`, `height`, `r`, `rx`) | literal | bound (`d={expr}`) | the element's run-time entry tessellated again, counted as `shapes_tessellated`; a literal shape's count stays 0 |
| Paint (`fill`, `stroke`) | its StyleKey's rules have no toggle or state selector and no `var()` that a binding sets | a `class:` toggle, a state pseudo-class or a bound custom property can change it | the cascade's normal restyle; the item's colour rewritten, no geometry |
| `stroke-width` | literal | from CSS that can change | a different stroke mesh: tessellated at run time like bound geometry |
| Transform | from layout or the 3D node | the node's transform changes | one uniform or instance write |
| `visibility`, `opacity` | as for any element | as for any element | no geometry |

Paint goes through the existing cascade: each shape element is a
template element with a `StyleKey`, so it shares a computed style with
every other element of the same share key (Analysis PR 2d). `fill`,
`stroke` and `stroke-width` become CSS properties in `vapors-css`, with
the presentation attributes mapped to them as Chromium maps them.

The analysis' per-property states for shapes come with the build-time
analysis' per-property work (finish-line item 7); until then every
shape is tracked at run time, as IU3 tracks boxes, and S5's table
already makes literal geometry free at run time. The analysis removes
tracking; it does not change what is drawn.

## 2. The racer on SceneRenderer

### 2.1 After 3D PRs 4, 4b and 5

PR 4 gives `<Scene>`, `<Group transform={..}>`, `<Mesh geometry={..}>`
and `<Camera>` in `.wisp` with reactive transforms, a signal change
writing one node's uniform. PR 4b feeds each node's anchor element's
computed `color`, `opacity` and `visibility` to its node, with `:hover`
through a 3D hit test. PR 5 keeps keyed nodes across reorder and
releases geometry on unmount.

### 2.2 What moves

| Racer today | One scene |
|---|---|
| `Track` renderer, its own pipelines (`gpu.rs`, `gpu/setup.rs`, `track.wgsl`) | `SceneRenderer` on the same `gpu-surface` named `track`; `Track` and `track.wgsl` deleted |
| Hand-written tilted orthographic projection in `vs_solid` | a `<Camera>` with `Projection::Orthographic { height, .. }` under a group carrying the tilt, yaw and look-ahead; `height` from the racer's zoom |
| Simulation X, Y ground, Z up | one root `<Group>` rotating -90 degrees about X, so the racer's geometry keeps its coordinates and the scene stays +Y up |
| Car flag in the vertex and heading in the shader | `<Group transform={car}>` holding the car's meshes; one uniform write per frame |
| Face shading baked into vertex colours | decision D4 |
| Procedural ground in the fragment shader | decision D5 |
| Brake lights brightened by a `brake` uniform | `class:braking={brake}` on the brake-light meshes' anchors; their `color` from CSS |

The ground and object check from 3D slice 2 (`3d-2.md`) is run on the
racer: the car's base and the ground under it land on the same pixel,
as `Projection::project` says, in Fixed and Follow camera.

The racer's existing tests move with it: resize and depth reallocation
(`gpu_track_resize_tests.rs`), unmount (`gpu_track_unmount_tests.rs`),
the IU5 per-frame counts (`gpu_hud_iu5_tests.rs`), and the browser
scripts named in `docs/RACER_PLAN.md`.

### 2.3 SVG markings on the craft

A scene component `<Decal transform={..} width={..} height={..}>` takes
an inline `<svg>` as its child. Its shapes are drawn in the 3D pass, in
the decal's plane in the parent node's space, with the viewBox mapped
onto the decal's width and height (decision D2). The craft carries a
roundel with a number and two stripes on its bonnet, written as SVG in
the racer's `.wisp`. Each `<path>` is a template element, so its `fill`
comes from CSS: a livery class on the scene root recolours the markings
and the body together.

The decal sits on the bonnet's top face (z = 9 in `gpu/mesh.rs`, the
body's 3 + 6); to keep it from fighting the face for depth it is drawn
with a depth bias or a small offset, chosen in S7 and tested at the
far end of the camera's zoom.

Coverage is computed in the 3D pass after the view projection, by the
method S3 chooses (the brief, "Antialiasing after every transform").
If S3 chooses MSAA, the scene surface needs a multisampled colour and
depth target and a resolve; analytic coverage needs neither.

### 2.4 CSS reaching scene nodes

Through PR 4b's route, with one stylesheet for HUD, scene and markings:

- a token `--craft-body` for the body colour, read by the body meshes;
- a livery enum on the scene root (`class:livery-a`, `class:livery-b`
  from an enum signal) that changes the body, stripe and number
  colours, set from a control in the car-setup panel;
- `class:braking` on the brake lights;
- `:hover` on the craft through PR 4b's 3D hit test, raising the
  body's brightness while the pointer is over it.

Each change is counted: no geometry uploaded, one uniform per changed
node or shape, `elements_restyled` equal to the anchors whose style
changed.

### 2.5 HTML on top

Unchanged from 3D slice 1c: the HUD is ordinary elements painted after
the `gpu-surface`, hit first (`docs/plans/3d-1c.md`, PR 5). The
acceptance re-runs those tests on the ported racer.

## 3. The 60 s frame-time run

### 3.1 What exists

`vapors-perf` (`crates/perf`) runs a workload's `App::frame` on an
offscreen texture: no surface, no present, no vsync
(`baseline-2026-11-10.md`, Setup). The frame clock advances 16 ms per
frame (`sampler.rs`, `INTERVAL`). After each frame, untimed, the sampler
waits for the GPU (10 s deadline), so frames do not overlap. Per frame
it keeps CPU wall time, phase times, the GPU span (two timestamps around
the frame, `GpuClock`) and the GPU time of render passes and surface
renders (the app's `GpuTimer`). Runs are capped at 2,048
(`args.rs`, `MAX_RUNS`: wgpu's `QUERY_SET_MAX_QUERIES` 4096 / 2), and
the racing workload presses Space and nothing else
(`crates/perf/src/racer.rs`), so the car does not move.

### 3.2 What the run adds

A workload `racer, one scene, 60 s`:

- **Length**: 60 s of the frame clock at its 16 ms step: 3,750 frames
  after 20 warmup frames.
- **Driving**: a scripted driver in the workload, deterministic: gas
  held, steering toward the next point on the circuit's centre line,
  brake into the bends, a camera switch between Fixed and Follow every
  10 s, a livery change at 30 s. When the race finishes three laps it
  restarts, so every frame is a racing frame. The lap time, and so the
  number of restarts, is to be measured.
- **Per frame, recorded**: CPU wall time and phases (as today); GPU
  time from the app's `GpuTimer`, which reads back per frame and has no
  cap; and frame-to-idle time, from the start of `App::frame` to the
  return of the sampler's GPU wait (decision D6). The span clock is not
  used in this mode, which lifts the 2,048 cap for it alone.
- **Judged**: the number of frames whose frame-to-idle time is over
  16.7 ms. The run exits non-zero when it is above 0, printing each such
  frame's index, simulated time, phase times and GPU time, and the
  scene's counts on that frame.
- **Reported**: median, p99 and max of CPU, GPU and frame-to-idle time;
  the over-budget count; draw calls, buffer bytes and atlas bytes per
  frame (median and max); load average at start and end.

### 3.3 Where and how it is run

As the finish line says for every property: release build, the dev
machine (Intel UHD 630, Mesa ANV, Vulkan, headless), scale 1,
1100 x 720 (the racer workloads' size), 1-minute load under 1.5, three
runs. Medians are reported across the runs; the property holds only
when each of the three runs has no frame over 16.7 ms. The result goes
in `docs/perf/` with the revision and the command.

What it does not measure: presentation. A windowed app with a Fifo
surface adds the acquire wait (`baseline-2026-11-10.md`, Setup); the
dev machine has no display. Phone and browser frame times stay with
the racer's manual browser scripts.

CI does not time anything (`baseline-2026-11-10.md`: "never asserted
in CI"; the gate hides `/dev/dri`, `docs/decisions/2026-11-02-gate-
hides-gpu-device.md`). A unit test runs the workload's script for 60
frames on lavapipe and checks it drives, switches camera and restarts.

## 4. PR stack

Each row is under about 400 lines (estimate), built by a fresh agent,
each check proven by breaking what it guards. After each row the next
is checked against what it found.

| PR | Content | Acceptance | Depends on | Parallel with |
|---|---|---|---|---|
| P0 | `vapors-perf`: the 60 s mode on today's racer (3.2), the scripted driver, frame-to-idle time, the over-budget refusal | Three runs on the dev machine of today's racer recorded in `docs/perf/`; the refusal shown failing on a frame made slow on purpose, then restored | #240 merged (same `report.rs`) | everything |
| S1 | `vapors-shapes`: the path parser moved from `vapors-text`, `ShapeTable`/`ShapeId` types; the icon route on the moved parser | Every icon test and the icon Chromium comparison unchanged; one parser (the old file gone) | none | 3D 4, 4b, 5; IU4 except PR 3 (`crates/text`), so after or before IU4 PR 3, not during |
| S2 | Classifier and tessellator (lyon, pinned; `cargo xtask deny` passing) in `vapors-shapes`, canonical form and content hash; C and Q curves in the parser | Unit tests: a rect path classified as a rect, two arcs as a circle, other paths as meshes; equal shapes one hash, a changed stroke width another; a non-finite or degenerate input refused naming the shape | S1 | everything |
| S3a | `Item::Shape`, the shape pipelines in `crates/renderer` and the painter's dispatch, on IU4's retained path | Shape runs batched in paint order; idle frame writes 0 shape bytes (IU4's counter) | S2, IU4 PR 2 | 3D 4, 4b, 5 |
| S3b | The antialiasing test (the brief's slice 1): touche's ten icons as shapes at 1x, 2x, rotated 30 degrees and scaled 1.37, MSAA against analytic coverage, against Chromium with the paint_check rules, beside the icon route | The numbers recorded; the method chosen, or the question returned to Tom if neither is within the rules at 1x (the brief, decision 3) | S3a | everything |
| S4 | Templates: `<path>` fill and stroke, `<rect>`, `<circle>`, `fill`/`stroke`/`stroke-width` in `vapors-css` with presentation attributes mapped; parsed and tessellated at run time, cached by hash | Chromium paint comparisons for fill, stroke and a class toggle recolouring; a toggle tessellates nothing (counted) | S3b | 3D 4, 4b, 5 |
| S5 | Compile time: the compiler parses, refuses, classifies, deduplicates, emits each crate's `SHAPES` and each element's `ShapeId`; the runtime registry merging by hash | A page of literal shapes: `shapes_tessellated` 0 on every frame; the same path in two components is one entry; a bad `d` fails the build naming file and line | S4 | 3D 4, 4b, 5 |
| S6 | Known-variable geometry: `d={expr}` tessellated on change, counted | Changing the signal tessellates that element once; others 0 | S5 | everything after S5; not on property 9's path |
| R1 | Racer ground geometry (D5) as Rust meshes from the circuit's constants in `model.rs` | Unit tests: road, verge and kerb at the distances `road_distance` gives; no GPU | none | everything |
| R2 | Lit material (D4) in `vapors-scene3d` and `-gpu`: normals with positions, one directional light | Pixel tests: a cube's faces at the expected shades; the ground and object check still passes | 3D PR 5 (shares `meshes.rs`) | IU4, S rows |
| R3a | The racer's world in `.wisp` on `SceneRenderer`: root group, camera, ground, scenery, car group; `Track` and `track.wgsl` deleted | The ground and object check on the racer in both cameras; the resize, unmount and IU5 tests moved and passing; browser scripts pass | R1, R2, 3D PRs 4 and 5 | S rows |
| R3b | CSS on the craft: `--craft-body`, the livery enum, `class:braking`, `:hover` (2.4) | Pixel tests per rule; each change counted: no geometry, one uniform per changed node | R3a, 3D PR 4b | S rows |
| S7 | Shapes in the 3D pass: `SceneRenderer` draws registered entries with a node's world matrix, paint from the anchor's style; depth bias | A decal on a plane rotated 60 degrees under a perspective camera: edge coverage compared with a reference render at 8x; no depth fighting at the racer's widest zoom | S5, 3D PR 4b | R3a, R3b |
| S8 | `<Decal>` with an inline `<svg>` in `.wisp` | A decal's paths take class toggles and tokens; a toggle writes one uniform, tessellates nothing | S7 | R3b |
| R4 | The craft's markings (2.3) | Pixel test: the roundel follows the car through a turn; livery change recolours body and markings in one frame | S8, R3b | none |
| P1 | Acceptance: P0's run on the one-scene racer, three runs, `docs/perf/`; property 9 marked held or not in `perf-finish-line.md` | No frame over 16.7 ms in each of the three runs, or the frames that were, with their phases | R4 | none |

Critical path: 3D 4/4b/5, then R2, R3a, R3b; S1 to S5 then S7, S8, R4;
then P1. P0, S1, S2 and R1 can start now. S3a waits for IU4 PR 2.

## 5. Open decisions for Tom

**D1. What "one scene" means.** (a) One document and one cascade: the
3D world drawn by `SceneRenderer` into its `gpu-surface`, HTML painted
over it by the painter, the surface composited as one item. (b) One
display list in which HTML boxes, shapes and meshes interleave, drawn in
one pass with one depth buffer. Recommendation: (a). It is what 3D
slices 1 and 2 build; (b) replaces the surface architecture and is a
new lane.

**D2. How SVG reaches a 3D node.** (a) `<Decal>`: the svg's shapes drawn
in the 3D pass from the shape table, coverage after the projection. (b)
The svg rendered to a texture used as a material (3D slice 3b, not
planned yet): coverage baked at the texture's resolution, soft up close
(the brief, "Composition"). Recommendation: (a); it is the brief's route
for crisp vectors and reuses S5's table.

**D3. Shape tables across crates.** (a) Per-crate tables, merged by
content hash at run time on first use. (b) One workspace-wide build
step that sees every crate's templates and emits one table. (c) No
cross-crate deduplication. Recommendation: (a): the compiler stays per
crate (`compile_dir` from each `build.rs`), and the merge costs one hash
lookup per entry once.

**D4. The racer's shading.** Today each face's shade is baked into its
vertex colour; `SceneRenderer` draws one unlit colour per mesh. (a) Pull
3D slice 3's basic-lit material forward (normals, one directional
light). (b) Unlit: every face of a mesh the same colour, a visible loss.
(c) Per-vertex colours in `Meshes`, which CSS could then not set.
Recommendation: (a); the brief's slice 3 builds it anyway.

**D5. The racer's ground.** Today it is computed per pixel in its own
shader with its own projection. (a) Meshes for grass, road, verge and
kerb built from the circuit's constants; centre line, start stripe and
grid boxes as SVG on decals laid on the ground; the per-pixel noise and
vignette dropped. (b) A custom ground material: a plane mesh with a
procedural fragment shader, needing a material extension point in
`SceneRenderer`. Recommendation: (a): one projection for everything, and
the ground's markings exercise the same shape route as the craft's.

**D6. What "no frame over 16.7 ms" measures.** (a) Frame-to-idle: from
the start of `App::frame` to the GPU finishing that frame's work, frames
not overlapping (the sampler's existing order). (b) CPU time and GPU
time each under 16.7 ms, as a pipelined display loop would allow. (c)
Wall time between presented frames in a windowed run, which this
headless machine cannot do. Recommendation: (a): the strictest of the
three, and measurable here. Today's racer is within it by the sum of
maxima (4.76 + 10.10 = 14.86 ms, baseline run 2), with the span's idle
gaps included; P0 gives the measured figure.

## 6. Risks

- Lyon's tessellation of strokes may not match Chromium's joins at
  small sizes; S3b's comparison shows it before templates rely on it.
- The livery and `:hover` rules depend on PR 4b's 3D hit test, not yet
  designed in detail.
- The frame budget adds scene draws (ground, scenery, car parts,
  decals: to be measured in R3a and R4) to the 33 the racer draws
  today (`3d-1c.md`, PR 3).
