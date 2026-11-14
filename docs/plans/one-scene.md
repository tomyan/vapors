# One scene: HTML, SVG and 3D (finish-line property 9)

Property 9 of `docs/plans/perf-finish-line.md`: the racer (3D craft, SVG
markings on it, HTML on top) drawn as one scene with CSS reaching scene
nodes, and no frame over 16.7 ms across a 60 s run. Order of work item
8: the shape table and the one-scene proof, after 3D slice 2's PRs 4,
4b and 5 (`docs/plans/3d-2.md`).

Read from the code at 7d1fe84. Plan only; nothing here is built. Line
counts per PR are estimates. Every number is cited or marked "to be
measured". Sections 1 to 4 follow the recommendation of each open
decision in section 5; where another option would change a row, the
decision says which.

## 0. Where things stand

- **Icons** (`docs/icons.md`): `<svg>` with stroked `<path>`s (M L H V A
  Z, `currentColor`, stroke only) is parsed at run time by
  `vapors_shapes::IconPath`, stroked on the CPU by zeno into an 8-bit mask
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
  triangle whose fragment shader computes road, verge, kerbs, markings,
  grass stripes, noise, shadows and a vignette per pixel (`track.wgsl`,
  `ground` and `fs_main`). Scenery and car are one vertex buffer of
  position, colour and a car flag, with face shading baked into the
  colours (`gpu/mesh.rs`), drawn in one call. The camera is a tilted
  orthographic view written by hand in the shader (`vs_solid`: no
  perspective divide, depth `0.5 - (...)/4000`). Simulation coordinates
  are X, Y on the ground and Z up. The `brake` value is written into the
  uniform (`gpu/setup.rs`, `car_values`) but no shader reads it, and the
  car has no brake-light meshes (`gpu/mesh.rs`, `car`).
- **3D slice 2** (`docs/plans/3d-2.md`): PRs 1 to 3b merged.
  `vapors-scene3d` holds `Scene` (generational node ids, `Group`,
  `Mesh { geometry, colour }`, `Camera`), +Y up, right-handed.
  `vapors-scene3d-gpu`'s `SceneRenderer` draws every mesh node unlit in
  one colour (`shader.wgsl`), single-sampled, one draw and one uniform
  slot per mesh node, the slot written only when its world version
  changes. PRs 4 (components), 4b (one cascade into the scene) and 5
  (lifecycle) are not started.
- **Racer frame cost today** (`docs/perf/baseline-2026-11-10.md`,
  section "IU3 PR 3c-1, after, full output", 200 frames, UHD 630):
  racing, CPU frame median 3.20 ms, max 3.59 ms; GPU span median 6.78
  ms, max 7.90 ms; GPU time median 4.71 ms, max 6.06 ms; 25 draw calls.
  The span starts before the frame is recorded and ends after the GPU
  finishes, so it covers CPU recording and GPU work together (same doc,
  after the "After one submit" table); frame-to-idle (D6) is therefore
  close to the span, not to CPU plus span.

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
- the table format and the process registry (1.3).

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
markings seen up close in 3D, is to be measured in S3b.

Antialiasing differs by kind. A closed-form shape has a distance to its
edge in the fragment shader, so analytic coverage from `fwidth` works
as it does for boxes. A tessellated mesh has interior triangle edges
with no edge distance; analytic coverage for it needs distances per
triangle, evaluated with `fwidth`, built at tessellation time but
evaluated after the transform. S2 builds them by edge
(`crates/shapes/src/mesh.rs`, the check; `mesh_edges.rs` in S2's last
PR, the builder): each triangle has its own three vertices, and for
an outline edge the two corners on it carry distance 0 and the third
its altitude, so the interpolated value is a fragment's signed
distance to that edge's line; for an edge two triangles share (found
by counting each edge's uses in lyon's output) all three corners
carry a far value, so it never wins the `min`. Fragments exist only
inside the mesh, which would leave the outer half of the coverage
ramp missing (every edge a hard edge on its outer side), so the
outline is extruded by half a device pixel in the vertex shader (Tom,
2026-11-13, option 2): each vertex carries the miter offset of its
outline edges as a normal and, per edge, the shift that move takes
off its distance; distances go negative outside the true outline and
the ramp straddles it, where Chromium's edge sits. The miter is taken
per triangle corner from the two outline edges bounding its wedge, so
where an outline touches itself at a vertex (a bowtie, an even-odd
star's points) each wedge moves out of its own side; two outline edges
meeting head-on (the end of a cut of no width into the shape) have no
miter and are refused, naming the vertex. Vertices lyon emits twice at
one position (a closed stroke's start) are welded first, so the edge
across the stroke there is shared, not a seam. Two shapes the builder
cannot draw are refused naming the shape and a vertex, until a path
that produces one is in hand as a fixture: an edge used by three or
more triangles, and a dropped sliver with triangles on both sides (a
T-junction inside the fill, whose edges would draw as outline).
Rejected: one
distance per shared vertex (every vertex of a convex fill, and every
vertex lyon's stroker emits, lies on the outline at zero, and a
corner triangle has two outline edges); an index buffer with
per-vertex edge flags (edge membership is per triangle); per-triangle
flags through `@interpolate(flat)` with an index buffer (WebGPU takes
the first vertex, so every triangle needs its own first vertex, extra
work for no bytes saved); barycentrics (not in WebGPU); curves
evaluated in the fragment shader, Loop-Blinn style (svg-support D1);
an inset-only ramp (the shape draws half a pixel small) and an AA
ring of inset fill plus a one-pixel band (an inset of a stroke under a
pixel wide folds its fill). Cost against the indexed layout it
replaces: a fill of n outline vertices is about 24n bytes indexed
(12-byte vertices, u32 indices) and 120n bytes here (40-byte vertices,
three per triangle, no index buffer), 5 times; a 24 px icon of 200
vertices about 24 KB against 5 KB; the vertex shader runs three times
per triangle. Small in absolute terms for icons; S5a measures. A known
limit for S3b: a pixel inside the shape within half a pixel of the
outline but inside a triangle with no outline edge (the wedge at an
outline vertex) gets coverage 1 where Chromium gives between 0.5 and
1, at most one slightly dark pixel per outline vertex, expected along
flattened curves. A CPU fringe of fixed width is early antialiasing
and is not a candidate (the brief, "Antialiasing after every
transform"). S3b compares, for each kind, MSAA against that kind's
analytic method.

A literal outside the subset is refused at build time naming file, line
and the fix, as the compiler refuses other attributes today
(`codegen/attribute_refusals.rs`).

### 1.3 Deduplicated and packed

- Two shapes are the same entry when their canonical form is equal:
  absolute commands in viewBox units after parsing, plus the fill rule
  and stroke parameters that change the mesh (width, join, cap,
  miter limit). Paint is not part of the key; it is per item. Equality
  is per build target: an arc whose radii the parser scaled up for its
  chord passes through `sin_cos`, whose last bit differs between the
  host's libm and wasm32's, so S4a's run-time parse must not rely on
  cross-target bit equality: on a hash hit it compares bytes, and a
  miss is a cache miss (one more tessellation, counted), never a
  defect (S2, `crates/shapes/src/canonical.rs`).
- Each crate's build emits one `static SHAPES: ShapeTable` beside its
  `wisp.rs`: triangle buffers for every mesh entry, parameters
  for every closed-form entry, and each entry's canonical bytes and
  their hash. Each shape element's emitted code names its index in that
  table.
- The compiler runs per crate from `build.rs` (`build/mod.rs`,
  `compile_dir`), so it cannot see another crate's shapes. This departs
  from the brief's one table for the app (decision D3). A process
  registry takes a crate's table when a component from that crate is
  first mounted, inside that frame's update, and merges entries: equal
  hashes are compared by canonical bytes once per entry, and a hash
  match with different bytes is refused, naming both crates and both
  sites. The registry issues the `ShapeId`s. The merge is O(entries)
  once per crate, inside a timed frame (D7).

### 1.4 What an item is at run time, and who holds its GPU buffers

`Item::Shape` in the 2D display list:

- `shape`: a `ShapeRef`, either a registered `ShapeId` or a run-time
  entry (1.5);
- `transform`: viewBox to device pixels, 2 x 3 affine;
- `paint`: the fill or stroke colour from the element's computed style;
- the clip, from the `Scene`'s clip as for every item.

Shapes join the painter's batching like quads: one pipeline per kind
(closed-form, mesh), instanced, in paint order. On IU4's retained path
(`docs/plans/iu4.md`, PR 2) a shape's instance bytes are kept between
frames like a quad's, so S3a1 lands after IU4 PR 2.

**GPU buffers.** A `ShapeStore`, one per device, owned by the
`ScenePainter` in `vapors-app`. At the start of each frame it uploads
every registered entry it has not uploaded (counted as
`shape_bytes_uploaded`; 0 on a frame that registers nothing). The
painter draws from it, and `SurfaceFrame` gains `shapes: &ShapeStore`
so a `SurfaceRenderer` such as `SceneRenderer` draws the same entries
(S7). A draw naming an entry the store has not uploaded is refused with
an error naming the entry and its site, never skipped. Added in S3a1
(the store and the 2D path) and S7 (the `SurfaceFrame` field).

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
already makes literal geometry free at run time.

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
| `Track` renderer, its own pipelines (`gpu.rs`, `gpu/setup.rs`, `gpu/pipeline.rs`, `track.wgsl`) | `SceneRenderer` on the same `gpu-surface` named `track`; those files deleted |
| Hand-written tilted orthographic projection in `vs_solid` | a `<Camera>` with `Projection::Orthographic { height, .. }` under a group carrying the tilt, yaw and look-ahead; `height` from the racer's zoom |
| Simulation X, Y ground, Z up | one root `<Group>` rotating -90 degrees about X, so the racer's geometry keeps its coordinates and the scene stays +Y up |
| Car flag in the vertex and heading in the shader | `<Group transform={car}>` holding the car's meshes; one uniform write per frame |
| Face shading baked into vertex colours | a per-vertex shade factor times the node's CSS colour (D4) |
| All scenery in one vertex buffer, one draw | scenery merged per colour: one mesh for every trunk, one for every tree's foliage, two for the building (base and roof). `scenery()` places 541 trees (cells of the 25 x 25 grid passing its `road_distance` and 250-unit tests); one node per tree part would be about 1,082 draws |
| Car: wheels, body, cabin, roof, two stripe cuboids (`gpu/mesh.rs`, `car`) | the same as meshes, the four wheels merged into one; the stripes replaced by R4's decal |
| Procedural ground in the fragment shader | a ground material on one plane mesh (D5) |
| No brake lights (the `brake` uniform is unread) | two new brake-light meshes at the car's rear, `class:braking={brake}` on their anchors, their `color` from CSS (R3b) |

That is about 11 scene draws before decals: ground 1, scenery 4, car 6
(wheels, body, cabin, roof, two brake lights). R3a1 pins the count.

The ground and object check from 3D slice 2 (`3d-2.md`) is run on the
racer: the car's base and the ground under it land on the same pixel,
as `Projection::project` says, in Fixed and Follow camera.

Racer tests that move to `SceneRenderer` (R3a2): `gpu_track_tests.rs`,
`gpu_track_idle_tests.rs`, `gpu_track_lifecycle_tests.rs`,
`gpu_track_resize_tests.rs`, `gpu_track_scale_tests.rs`,
`gpu_track_unmount_tests.rs`, `gpu_track_unmount_order_tests.rs`,
`gpu_hud_iu5_tests.rs`, `gpu_hud_iu5_probe.rs`, and `gpu/mesh_tests.rs`
(to the new mesh builders). The browser scripts named in
`docs/RACER_PLAN.md` are re-run by hand, as today.

### 2.3 SVG markings on the craft

A scene component `<Decal transform={..} width={..} height={..}>` takes
an inline `<svg>` as its child. Its shapes are drawn in the 3D pass, in
the decal's plane in the parent node's space, with the viewBox mapped
onto the decal's width and height (decision D2). The craft carries a
roundel with a number and two stripes on its bonnet, written as SVG in
the racer's `.wisp`; the number is drawn as paths, since `<text>` is the
brief's slice 7 and not in this plan. The decal replaces the car's two
stripe cuboids. Each `<path>` is a template element, so its `fill` comes
from CSS: a livery class on the scene root recolours the markings and
the body together.

The decal sits on the bonnet's top face (z = 9 in `gpu/mesh.rs`, the
body's 3 + 6); to keep it from fighting the face for depth it is drawn
with a depth bias or a small offset, chosen in S7 and tested at the
far end of the camera's zoom.

Coverage is computed in the 3D pass after the view projection, by the
method S3b chooses. If S3b chooses MSAA, the scene surface needs a
multisampled colour and depth target and a resolve; analytic coverage
needs neither.

### 2.4 CSS reaching scene nodes

Through PR 4b's route, with one stylesheet for HUD, scene and markings:

- a token `--craft-body` for the body colour, read by the body meshes;
- a livery enum on the scene root, bound to one `class:` per variant,
  that changes the body, stripe and number colours, set from a control
  in the car-setup panel;
- `class:braking` on the new brake lights;
- `:hover` on the craft through PR 4b's 3D hit test, raising the
  body's brightness while the pointer is over it.

Each change is counted: no geometry uploaded, one uniform per changed
node or shape, `elements_restyled` equal to the anchors whose style
changed.

### 2.5 HTML on top

Unchanged from 3D slice 1c: the HUD is ordinary elements painted after
the `gpu-surface`, hit first (`docs/plans/3d-1c.md`, PR 5). Under D1(a)
this part of property 9 holds today; the new content is SVG in the 3D
pass and CSS reaching scene nodes. Property 9 does not promise HTML
placed inside the 3D scene.

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
(`args.rs`, `MAX_RUNS`: wgpu's `QUERY_SET_MAX_QUERIES` 4096 / 2), every
workload runs 20 untimed warmup frames by default, and the racing
workload presses Space and nothing else (`crates/perf/src/racer.rs`),
so the car does not move.

### 3.2 What the run adds

A workload `racer, one scene, 60 s`:

- **Length**: 60 s of the frame clock at its 16 ms step: 3,750 frames,
  judged from the first frame after the racer is made, with no warmup
  (D7). The first 20 frames' times are also printed on their own, so
  first-use costs (mount, pipeline creation, geometry upload, shape
  registration) are visible.
- **Driving**: a scripted driver in the workload, deterministic: gas
  held, steering toward the next point on the circuit's centre line,
  brake into the bends, a camera switch between Fixed and Follow every
  10 s of simulated time, a livery change at 30 s. When the race
  finishes three laps it restarts, so every frame is a racing frame. A
  lap is about 1,662 units (4 x `STRAIGHT` 180 + 2 pi x `RADIUS` 150,
  `model.rs`), so three laps take at least 22.7 s at `MAX_SPEED` 220
  and the run restarts at most twice; the actual lap time is to be
  measured.
- **Per frame, recorded**: CPU wall time and phases (as today); GPU
  time from the app's `GpuTimer`, which reads back per frame and has no
  cap; and frame-to-idle time, from the start of `App::frame` to the
  return of the sampler's GPU wait (D6). The span clock is not used in
  this mode, which lifts the 2,048 cap for it alone.
- **Refused, exiting non-zero**:
  - any frame whose frame-to-idle time is over 16.7 ms, printing each
    such frame's index, simulated time, phase times, GPU time and the
    scene's counts;
  - any frame whose GPU reading is missing (`GpuTime` other than
    `Read`, or a reading that names the deadline), naming the frame; a
    missing reading is never counted as 0 or left out of the statistics;
  - a device without `TIMESTAMP_QUERY`, naming it, before the first
    frame;
  - a 1-minute load average of 1.5 or more at the start or the end,
    printing it.
- **Reported**: median, p99 and max of CPU, GPU and frame-to-idle time;
  the over-budget count; draw calls, buffer bytes, atlas bytes and
  `shape_bytes_uploaded` per frame (median and max); load average at
  start and end.

### 3.3 Where and how it is run

As the finish line says for every property: release build, the dev
machine (Intel UHD 630, Mesa ANV, Vulkan, headless), scale 1,
1100 x 720 (the racer workloads' size), three runs. Medians are
reported across the runs; the property holds only when each of the
three runs has no frame over 16.7 ms. The result goes in `docs/perf/`
with the revision and the command.

What it does not measure: presentation. A windowed app with a Fifo
surface adds the acquire wait (`baseline-2026-11-10.md`, Setup); the
dev machine has no display. Phone and browser frame times stay with
the racer's manual browser scripts.

CI does not time anything (`baseline-2026-11-10.md`: "never asserted
in CI"; the gate hides `/dev/dri`, `docs/decisions/2026-11-02-gate-
hides-gpu-device.md`). Two tests cover the workload without timing:

- a unit test, no GPU, of the driver's schedule against simulated time:
  it steps the race model through the driver alone and checks the
  camera switch at 10 s and 20 s, the livery change at 30 s, and the
  restart after three laps;
- a GPU test on lavapipe of 60 frames (0.96 s): the car's speed and
  position change, and every frame asks for the next.

## 4. PR stack

Added by Tom's decision, 2026-11-13 (from P0's runs,
docs/perf/scene-run-2026-11-13.md: frame 0 about 98 ms with a cold Mesa
shader cache and about 22 ms warm, every later frame under 16.7 ms):
row F0, the first frame. Every pipeline is built before frame 0, from
one registry of vapors' pipelines; the glyph atlas's first upload and
the surface's first setup move before frame 0. Accepted when: P0's run
shows frame 0 under 16.7 ms cold and warm on the dev machine, and a test
fails when a pipeline is created during a judged frame, so later rows'
pipelines must join the registry. Start-up time is moved out of the
judged frames, not reduced. A vapors-owned on-disk pipeline cache
(faster later launches; wgpu supports it on Vulkan only) is deferred to
after the finish line (Tom, 2026-11-13).

Each row is under about 400 lines (estimate), built by a fresh agent,
each check proven by breaking what it guards. After each row the next
is checked against what it found.

| PR | Content | Acceptance | Depends on | Parallel with |
|---|---|---|---|---|
| P0 | `vapors-perf`: the 60 s mode on today's racer (3.2), the scripted driver, frame-to-idle time, the four refusals, the schedule unit test | Three runs on the dev machine of today's racer recorded in `docs/perf/`; each refusal shown failing (a frame made slow, a GPU reading withheld, load faked high), then restored | #240 merged (same `report.rs`) | everything |
| S1 | `vapors-shapes`: the path parser moved from `vapors-text`, `ShapeTable`/`ShapeId` types; the icon route on the moved parser. Two PRs: #278 the move; the types (`ShapeTable::check` refuses an undrawable geometry or a shared hash, naming crate and site) in the second | Every icon test and the icon Chromium comparison unchanged; one parser (the old file gone) | none | 3D 4, 4b, 5; IU4 except PR 3 (`crates/text`), so after or before IU4 PR 3, not during |
| S2 | Classifier and tessellator (lyon, pinned; `cargo xtask deny` passing) in `vapors-shapes`, canonical form and hash, per-triangle edge distances for meshes (1.2); C and Q curves in the parser. Five PRs: curves, segments and a curve-icon paint fixture; canonical form and hash; the classifier; the mesh format; the tessellator | Unit tests: a rect path classified as a rect, two arcs as a circle, other paths as meshes; equal shapes one hash, a changed stroke width another; a non-finite or degenerate input refused naming the shape | S1 | everything |
| S3a1 | `Item::Shape`, the `ShapeStore` (1.4), the closed-form pipeline and the painter's dispatch, on IU4's retained path | Rects and circles batched in paint order; idle frame writes 0 shape bytes; a draw of an entry not uploaded refused naming it | S2, IU4 PR 2 | 3D 4, 4b, 5 |
| S3a2 | The mesh pipeline | Fill and stroke meshes batched in paint order with closed-form shapes | S3a1 | 3D 4, 4b, 5 |
| S3b | The antialiasing test (the brief's slice 1): touche's ten icons as shapes at 1x, 2x, rotated 30 degrees and scaled 1.37; per kind, MSAA against its analytic method (1.2), against Chromium with the paint_check rules, beside the icon route; the curve tolerance | The numbers recorded; a method chosen per kind, or the question returned to Tom if none is within the rules at 1x (the brief, decision 3) | S3a2 | everything |
| S4a | `<path>` fill and stroke in templates; `fill`, `stroke`, `stroke-width` in `vapors-css` with the presentation attributes mapped; run-time parse and tessellation cached by hash | Chromium paint comparisons for fill and stroke; a class toggle recolours and tessellates nothing (counted) | S3b | 3D 4, 4b, 5 |
| S4b | `<rect>` and `<circle>` | Chromium paint comparisons, including a rounded rect | S4a | 3D 4, 4b, 5 |
| S5a | Compile time: the compiler parses, refuses, classifies, deduplicates and emits each crate's `SHAPES` and each element's index | A bad `d` fails the build naming file and line; the same path in two components of one crate is one entry (snapshot test) | S4b | 3D 4, 4b, 5 |
| S5b | The process registry (1.3): merge on first mount, the byte comparison and its refusal, `ShapeId`s to the elements | A page of literal shapes: `shapes_tessellated` 0 on every frame; the same path in two crates is one entry; a forged hash collision refused naming both crates | S5a | 3D 4, 4b, 5 |
| S6 | Known-variable geometry: `d={expr}` tessellated on change, counted | Changing the signal tessellates that element once; others 0 | S5b | everything after S5b; not on property 9's path |
| R1 | The shade factor (D4): a per-vertex shade in `Meshes` geometry, multiplied by the node's colour in `shader.wgsl` | Pixel tests: a cube with shades 0.65 and 1.0 on two faces under one CSS colour; the ground and object check still passes | 3D PR 5 (shares `meshes.rs`) | IU4, S rows |
| R2 | The ground material (D5): an application material on a mesh node (a WGSL fragment given by the app, reading world position from the scene's view-projection); the racer's ground shader ported to it | The racer's ground drawn through it matches `track.wgsl`'s colours at sample points on road, kerb, verge and grass, within 1 per channel | R1 | IU4, S rows |
| R3a1 | The racer's world in `.wisp`: root group, camera, ground, merged scenery, car group, on a test page beside the racer | The ground and object check on the racer's geometry in both cameras; scene draw calls pinned (about 11, 2.2) | R2, 3D PRs 4 and 5 | S rows |
| R3a2 | The racer moves to it; `Track` and its files deleted; the tests in 2.2 moved | Every moved test passes; idle frames render nothing; browser scripts pass by hand | R3a1 | S rows |
| R3b | CSS on the craft: `--craft-body`, the livery enum, brake-light meshes with `class:braking`, `:hover` (2.4) | Pixel tests per rule (the brake lights brighten while braking); each change counted: no geometry, one uniform per changed node | R3a2, 3D PR 4b | S rows |
| S7 | Shapes in the 3D pass: `SurfaceFrame::shapes`, `SceneRenderer` draws registered entries with a node's world matrix, paint from the anchor's style; depth bias | A decal on a plane rotated 60 degrees under a perspective camera: edge coverage compared with a reference render at 8x; no depth fighting at the racer's widest zoom | S5b, 3D PR 4b | R3a, R3b |
| S8 | `<Decal>` with an inline `<svg>` in `.wisp` | A decal's paths take class toggles and tokens; a toggle writes one uniform, tessellates nothing | S7 | R3b |
| R4 | The craft's markings (2.3), replacing the stripe cuboids | Pixel test: the roundel follows the car through a turn; a livery change recolours body and markings in one frame | S8, R3b | none |
| P1 | Acceptance: P0's run on the one-scene racer, three runs, `docs/perf/`; property 9 marked held or not in `perf-finish-line.md` | No frame over 16.7 ms in each of the three runs, or the frames that were, with their phases | R4 | none |

Critical path: 3D 4/4b/5, then R1, R2, R3a1, R3a2, R3b; S1 to S5b then
S7, S8, R4; then P1. P0, S1 and S2 can start now. S3a1 waits for IU4
PR 2.

If D5 goes to (a), R2 becomes three rows: ground meshes from the
circuit's constants, stroke dashes in `vapors-shapes` (the broken centre
line and the alternating kerb), and the ground decal.

## 5. Decisions

Decided by Tom, 2026-11-12, taking the lead's recommendations: D1 (a),
D2 (a), D3 (a) per-crate tables merged by hash (a departure from the
brief's one table for the app), D4 (d) shade times CSS colour, D5 (b)
ground material, D6 (a) frame-to-idle, D7 (a) every frame judged, no
warm-up. The options are kept below as the record of what was weighed.

**D1. What "one scene" means.** (a) One document and one cascade: the
3D world drawn by `SceneRenderer` into its `gpu-surface`, HTML painted
over it by the painter, the surface composited as one item. Under (a),
"HTML on top" already holds; what property 9 adds is SVG in the 3D pass
and CSS reaching scene nodes. (b) One display list in which HTML boxes,
shapes and meshes interleave, drawn in one pass with one depth buffer,
which would also allow HTML placed in the scene. Recommendation: (a).
It is what 3D slices 1 and 2 build; (b) replaces the surface
architecture and is a new lane.

**D2. How SVG reaches a 3D node.** (a) `<Decal>`: the svg's shapes drawn
in the 3D pass from the shape table, coverage after the projection. (b)
The svg rendered to a texture used as a material (3D slice 3b, not
planned yet): coverage baked at the texture's resolution, soft up close
(the brief, "Composition"). Recommendation: (a); it is the brief's route
for crisp vectors and reuses S5's table.

**D3. Shape tables across crates.** The brief's direction (agreed
2026-11-10) is one table for the app, built by a compiler that sees
every template. The compiler runs per crate, from each `build.rs`.
(a) Per-crate tables, merged by hash and canonical bytes when a crate's
first component mounts, inside that frame. This departs from the
brief. (b) The brief's design: one workspace-wide build step that reads
every crate's templates and emits one table, a new build mechanism
beside `compile_dir`. (c) No cross-crate deduplication.
Recommendation: (a): it keeps one build mechanism, and the merge is
O(entries) once per crate; its cost lands in the first frames, which
D7 decides whether to judge.

**D4. The racer's shading.** Today each face's shade is baked into its
vertex colour; `SceneRenderer` draws one unlit colour per mesh.
(a) Pull 3D slice 3's basic-lit material forward (normals, one
directional light). (b) Unlit: every face of a mesh the same colour, a
visible loss. (c) Per-vertex colours in `Meshes`, which CSS could then
not set. (d) A per-vertex shade factor (today's 0.65 to 1.0) multiplied
in the shader by the node's colour from CSS: today's look, CSS control,
no normals or light; the shade does not follow the camera or a light,
which the racer's fixed tilt does not need. Recommendation: (d): the
smallest change that keeps the look and CSS together, and it does not
wait for slice 3.

**D5. The racer's ground.** Today it is computed per pixel in its own
shader with its own projection.
(a) Meshes and shapes: a grass plane mesh; road, verge, kerb and
shoulder lines as SVG strokes of the circuit's centre line on a ground
decal, antialiased by the shape route; start stripe and grid boxes as
SVG. This needs stroke dashes, which are not in the subset, for the
broken centre line and the alternating red and white kerb. It loses the
grass stripes, the noise, the vignette and the shadows under the
building and the 541 trees unless each is drawn as more shapes. Any
edge drawn as a plain `SceneRenderer` mesh is aliased, since the scene
pass is single-sampled and has no late antialiasing for meshes.
(b) A ground material: one plane mesh drawn by an application-given
WGSL fragment that reads world position from the scene's
view-projection, porting `ground()` with its one-pixel `smoothstep`
edges. It keeps every part of today's look. The cost is an
application-material extension point in `SceneRenderer` (the 3D brief
asks to preserve one for application rendering); with D4(d) or (a)
the renderer already gains a second shading path, so the extension is
one pipeline more. Its markings are not SVG and not styled by CSS.
Recommendation: (b): it keeps the ground's look at the cost of one
pipeline, and property 9 promises nothing about the ground; the craft's
decal already proves SVG in the 3D pass.

**D6. What "no frame over 16.7 ms" measures.** (a) Frame-to-idle: from
the start of `App::frame` to the GPU finishing that frame's work, frames
not overlapping (the sampler's existing order). (b) CPU time and GPU
time each under 16.7 ms, as a pipelined display loop would allow. (c)
Wall time between presented frames in a windowed run, which this
headless machine cannot do. Recommendation: (a): the strictest of the
three, and measurable here. Today's racing racer's span, which bounds
frame-to-idle from above, has a max of 7.90 ms (section 0); P0 gives
the measured figure.

**D7. Which frames are judged.** The run's first frames hold first-use
costs: the surface's mount and pipeline creation (on the first frame
that paints it, `docs/gpu-surface.md`, "Renderer contract"), geometry
upload, and the shape registry's merge (1.3). A user sees those frames.
(a) Judge every frame from the first, no warmup; if the first frames
fail, the work is moving first-use costs out of them (for example,
pipelines made before the first frame is shown), not excluding them.
(b) Exclude the first N frames from the judgement, N named in
`perf-finish-line.md` row 9, and print their times. (c) Exclude only
the frame that mounts the surface. Recommendation: (a): the property
says no frame, and today's racer has never been measured from its
first frame, so P0 shows whether (a) is reachable before anything is
built for it.

## 6. Risks

- Lyon's tessellation of strokes may not match Chromium's joins at
  small sizes; S3b's comparison shows it before templates rely on it.
- The livery and `:hover` rules depend on PR 4b's 3D hit test, not yet
  designed in detail.
- Pipeline creation on ANV on the first frame may alone exceed 16.7 ms
  (not measured); under D7(a) that fails the property until moved.
- The frame budget adds scene draws (about 11 meshes, 2.2, plus decal
  shapes: to be measured in R3a1 and R4) to the 25 the racer draws
  today (`baseline-2026-11-10.md`, after ed08432).
