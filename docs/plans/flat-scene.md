# Flat scene: one primitive list, global optimisation, late antialiasing

Design only, for after the performance finish line
(`docs/plans/perf-finish-line.md`). Nothing here is built and nothing
here changes current work. Read from `origin/main` at adb3b677
(2026-11-13: IU4 PRs 1 to 5 merged, splice A and B merged, 3D slice 2
PRs 1 to 3b merged and PR 4 open). No cargo command was run for this
document; every number is cited to a file or marked "to be measured"
with the method, and every outside claim is cited to a source read on
2026-11-13 or marked "not verified".

Tom's direction (2026-11-13), which this plan is written to: flatten
everything (HTML, SVG, 3D) into one list and optimise it globally;
antialias late; handle text specially, above all at small sizes; keep
crisp pixel snapping where content is pixel-aligned so nothing is
blurry without reason; and recognise that once any projection,
rotation, scale or non-integer translate breaks pixel alignment,
snapping and hinting become counterproductive (jitter and shimmer under
animation, worse text), so the design must switch paths cleanly.

Standing rules it is bound by: analysable sources only
(`docs/decisions/2026-11-11-analysable-sources.md`); a small shared
runtime, never per-use copies; runtime performance first (idle does
nothing; costs flat in page size; `perf-finish-line.md`); every failure
loud; correctness checked against Chromium through the existing
oracles; 3D may become suitable for games and is never ruled out
(`docs/plans/particles.md`, "Decided by Tom").

## Contents

0. Where things stand, and what "flat" changes
1. One primitive model: boxes, text, shapes, meshes; 2D order against 3D depth
2. Global optimisation over the flat list, and what it replaces
3. Compile-time invariants in place of run-time checks
4. Antialiasing: late resolve, text, exact snapping, and the switch
5. Costs: measured, estimated, and how each is measured
6. Generality: off-path workloads for vapors-perf
7. Relation to the existing plans: keep, change, drop
8. Slices
9. Decisions for Tom
10. Risks
11. Sources

## Summary

- Today vapors has two renderers in one frame: the 2D painter over a
  display list of six item kinds in scroll-frame coordinates, and a 3D
  `SceneRenderer` drawing into its own texture in its own pass, composed
  as one more 2D item (section 0). One-scene D1 (a) keeps that for
  property 9. This plan is the design for D1 (b), one list, for after
  the finish line.
- One primitive record (shape, transform, paint, clip, order; section
  1) covers boxes, shadows, text, images, SVG shapes and 3D meshes. 2D
  paint order and 3D depth meet through one order key: a 3D scene is one
  place in its parent's paint order and a depth range of the frame's one
  depth attachment inside, so the whole frame is one render pass.
- The global pass over the list is what IU4 already built for 2D, made
  general: one cull (per context: the extents index for 2D frames, a
  frustum test for 3D contexts), one sort by order key, one batcher
  keyed by pipeline and bound resources, kept GPU bytes per batch. What
  changes: the transform moves from a per-frame dynamic-offset uniform
  to a table indexed per instance, and the clip from scissor state to
  data, so a batch can span frames and the clip can be a rounded
  rectangle (section 2).
- Antialiasing (section 4): analytic coverage in the fragment shader for
  closed forms and atlas text, as today; MSAA as the late resolve for
  meshes, turned on for a frame only when its page can draw one; three
  text regimes (snapped and hinted at rest, resampled while a translation
  or scale animates, outlines under rotation or projection) with the
  switch made at compile time for static transforms and at run time for
  animated ones, by the rule Chromium's compositor uses: fixed raster
  during an animation, re-raster at settle.
- Compile-time proofs (section 3) replace three kinds of run-time
  detection: a per-crate `PaintUses` table says which paths a page can
  need, so the unneeded ones are never built; marks the runtime trusts
  in release and checks in debug (the splice mark, a constant transform
  class, a constant clip); and constants folded into generated calls
  (shape ids, winners).
- Twelve workloads for `vapors-perf` (section 6) so an off-path page
  shows as a number: nested scrollers, wrapping flex, grid, thousands of
  small scrollers, mixed text, positioned elements, a translated card, a
  scaling card, icons, rounded clips, a scene with HTML, and a mixed
  page.
- Fourteen slices (section 8) each under about 400 lines, ordered so
  the first three (an explicit transform class, the transform table,
  clips as data with rounded `overflow: hidden`) are useful before any
  3D content enters the list. Ten decisions (section 9).

## 0. Where things stand, and what "flat" changes

Read from the code at adb3b677. Line numbers are from that revision.

| Part | Today | Where |
|---|---|---|
| 2D display list | `Item::{Shadow, Quad, Image, Icon, Surface, Text}`, device pixels, each under one scissor clip (`Option<Rect>`) | `crates/scene/src/display.rs:121-163`, `set_clip` line 190 |
| Coordinates | Each part in its scroll frame's coordinates; the frame's shift is whole device pixels; readers add it; `Frames::of` is O(scroll containers) per paint | `crates/elements/src/frames.rs:16-29`, `62-79` |
| Snapping | Every box edge `half_up(v * scale)` at paint, so a shared CSS edge is one device edge and a whole shift commutes with the snap | `crates/elements/src/paint.rs:112-136`; `docs/plans/iu4.md` section 3 |
| Batching | Consecutive items of one kind under one clip, per frame; 52 batches for the 17 visible rows of the rows page | `crates/scene/src/batch.rs:91-106`, `crates/elements/src/framed_batches.rs`; `iu4.md` section 1 |
| Culling | A per-frame extents index over drawing steps (`tops`, `reach`; tall steps in a list apart); candidates tested then sorted by place | `crates/elements/src/frame_extents*.rs`; `iu4.md` PR 5 as built |
| Kept GPU data | Per batch, a range in a `Retained` buffer; batches matched by key when made again; text bytes kept with atlas pins; the frame's shift in a dynamic-offset uniform | `crates/renderer/src/retained.rs`, `uniform_slots.rs`, `shift.rs`; `iu4.md` PRs 2, 3, 4c |
| Pipelines | quads, shadows, solid, image (`crates/renderer`), text and icons (`crates/text`); one render pass per frame, scissor per draw | `crates/renderer/src/*.wgsl`, `open_pass.rs` |
| Box coverage | Signed distance to the rounded outer edge, `clamp(0.5 - d, 0, 1)`: exact on a whole-pixel edge, one pixel of antialiasing on a curve | `crates/renderer/src/quads.wgsl:1-7`, `106-129` |
| Shadow coverage | Closed-form Gaussian of a rounded rectangle (erf along x, four samples along y) | `crates/renderer/src/shadows.wgsl` |
| Text | Shaped by parley; glyphs rasterised on the CPU by skrifa and zeno into an R8 atlas at one of 4 horizontal subpixel steps, baseline rounded to a whole pixel, light (vertical-only) autohinting below 2x and none at 2x; coverage to alpha by Skia's mask-gamma rule with Chromium's Linux parameters | `crates/text/src/raster.rs`, `raster_types.rs:10-27`, `quads.rs:75-82`, `text.wgsl:46-78`; `docs/text.md` "Rendering" |
| Icons | Stroked on the CPU by zeno into atlas masks at device size, one tinted quad per path; capped at 256 device px | `crates/elements/src/svg_paint.rs`, `crates/text/src/draw_icon.rs`; `docs/icons.md` |
| 3D | `SceneRenderer`: its own render pass into an `Rgba8Unorm` texture, `Depth32Float` cleared and discarded, single-sampled, unlit flat colour, one draw and one 80-byte uniform slot per mesh node, composed by the painter as `Item::Surface` with alpha blending | `crates/scene3d-gpu/src/draw.rs:59-100`, `shader.wgsl`, `slots.rs`; `docs/gpu-surface.md` "Renderer contract" |
| Transforms | CSS `transform` parses `translate`, `translateX`, `translateY`, `scale` and is refused when set (`Deferred`); nothing in 2D rotates or scales after layout; a `gpu-surface` refuses `transform` document-wide | `crates/css/src/syntax/special.rs:116-130`, `crates/elements/src/style/honoured.rs:143`; `docs/gpu-surface.md` "Supported composition" |
| Oracle | Chromium 155 through three pixel rules: flat neighbourhoods within 2, every pixel within 96, at most 2 strongly-off pixels per element region | `crates/elements-corpus/src/paint_check.rs:1-35` |
| Measured | Rows page draw 488 µs idle at 3,000 rows and 1,223 µs at 10,000 before IU4 PR 5; racer racing CPU 2,494 µs, GPU 4,763 µs, 25 draw calls; racer frame 0 about 98 ms cold and 22 ms warm, every later frame under 16.7 ms | `docs/perf/timed-2026-11-12.md`; `docs/perf/scene-run-2026-11-13.md` |

What "flat" changes, in one sentence each:

- The 3D scene stops being a texture composed by the painter and
  becomes a range of primitives in the painter's own list, drawn in the
  same pass with a depth range of its own (1.4).
- SVG shapes, 3D meshes and (later) text under a 3D node are the same
  primitive record with a 4x4 transform, so one-scene's S7 route
  (`SurfaceFrame::shapes`) is not needed (7.1).
- The frame's shift and the clip stop being pipeline state and become
  per-instance data, so batching is by pipeline and bound textures only
  (2.2, 2.3).
- Snapping becomes a decision per transform node, taken at compile time
  where the transform is constant and at run time where it animates,
  rather than a property of every item (4.3).
- The renderer's special cases become numbers: a workload per shape of
  page, each judged flat by `--scaling` or pinned (section 6).

What does not change: the element tree, the cascade, layout, placement,
the kept paint and its debug checks, the retained GPU buffers, the
atlas, and every Chromium oracle. Each slice keeps pixels unchanged on
every existing fixture, which the oracles and the kept-paint checks
(`kept_paint_check.rs`, `painter_kept_check.rs`) are there to prove.

## 1. One primitive model

### 1.1 The record

Every painted thing is one record. The fields are what `Item` has today
plus what SVG shapes (`one-scene.md` 1.4) and meshes need; nothing is
added for its own sake.

```
Primitive {
    shape:     Shape,          // what is covered (1.2)
    transform: TransformRef,   // index into the transform table (2.2)
    paint:     Paint,          // solid colour, gradient index, texture, material
    clip:      ClipRef,        // index into the clip table, or none (2.3)
    order:     Order,          // (context, place) (1.3)
}
```

- `Shape` is an enum of the closed forms the shaders evaluate
  (`RoundedRect { rect, radii, border }`, `Ellipse`, `Segment`, the
  shadow's blurred rounded rect), `Glyph { atlas rect, position }`,
  `Image { rect }`, `Mesh { geometry id, range }` for tessellated paths
  and 3D geometry alike, and `Outline { glyph id }` for text drawn as a
  shape (4.4). A closed form carries its own geometry in local units; a
  mesh names an entry in the shape table (`one-scene.md` 1.3) or the
  `Meshes` store (`crates/scene3d/src/meshes.rs`).
- `TransformRef` names a transform node (2.2): the page, a scroll
  frame, a lane row (`splice-layout.md` section 2), a transformed
  element, or a 3D node. The primitive's local coordinates are in that
  node's space: device pixels for 2D nodes (as a part's items are in its
  frame today), the app's units for 3D nodes.
- `Paint` is per primitive and never part of a shape entry, as
  one-scene 1.3 says for shapes.
- `ClipRef` names a clip node: a rectangle in some transform node's
  space, a rounded rectangle, or (later, `svg-support.md` V12) a
  coverage texture. Clip nodes nest; the table holds the chain.
- `Order` is the sort key (1.3).

What a primitive is not: a layer. Group opacity, filters and masks need
a subtree drawn to a texture (`svg-support.md` 3.4, F3's work). In the
flat list a layer is a context (1.3) whose primitives are drawn to a
pooled texture in an earlier pass and whose result is one `Image`
primitive in the parent context. Layers are the one thing that is not
flat, because compositing a subtree at an opacity has no per-primitive
form.

### 1.2 Kinds against pipelines

| Kind today | Shape | Pipeline | Coverage |
|---|---|---|---|
| `Quad` | `RoundedRect` with border widths and colours | quads (as `quads.wgsl`) | analytic, as today |
| `Shadow` | blurred `RoundedRect` | shadows | analytic, as today |
| `Text` (upright) | `Glyph` per inked glyph | text | atlas coverage through `adjust()` (`text.wgsl:59-71`) |
| `Icon` | closed form or `Mesh` from the shape table | shapes (one-scene S3a) | analytic or MSAA, per S3b |
| `Image` | `Image` | image | textured; edges on whole pixels when snapped |
| SVG `<path>`, `<rect>`, `<circle>` | closed form or `Mesh` | shapes | per S3b |
| 3D mesh node | `Mesh` with a 4x4 transform | mesh (one per material kind, `3d-engine-parity.md` G4) | MSAA when on (4.2) |
| Text under rotation or projection | `Outline` | shapes, with text's `adjust()` | analytic or MSAA; 4.4 |
| `Surface` (application renderer) | `Image` of the surface's texture | image | unchanged: the surface stays for application rendering (touche's image renderer); it stops being the 3D scene's host (7.1) |

One pipeline per (shape family, paint family, material kind); the
count is small and fixed by the `PaintUses` table (3.2), never per use.

### 1.3 Order: 2D paint order against 3D depth

An `Order` is `(context: u32, place: u32)`.

- A **context** is a range of the list drawn under one ordering rule.
  The page is context 0 and orders by CSS paint order (Appendix E as
  `paint_order.rs` reduces it): `place` is the drawing step's position
  in the kept order, as the extents index sorts candidates by place
  today (`iu4.md` section 4, "sorted by place").
- A **3D context** is a scene. It occupies one place in its parent
  context (the place its `<Scene>` anchor's box would have, which is
  where the `gpu-surface` item sits today), and inside it the order is
  the depth buffer's for opaque meshes and back-to-front by view depth
  for blended ones (`css-for-3d.md` D4; `3d-engine-parity.md` A1).
  `place` within a 3D context is the blend sort's rank, 0 for opaque.
- A **layer context** draws to a texture in an earlier pass and is
  placed as one `Image` in its parent.

Depth. The frame has one `Depth32Float` attachment (the format
`SceneRenderer` uses, `crates/scene3d-gpu/src/depth.rs`), present
whenever the page's `PaintUses` names a 3D context and absent
otherwise, so a page without 3D pays nothing. 2D primitives draw with
depth test `Always` and depth write off. Each 3D context is given a
slice of the depth range through the viewport's `min_depth` and
`max_depth` (`RenderPass::set_viewport` in wgpu, the WebGPU
`setViewport` depth range): context `k` of `N` scenes in the frame
maps its clip-space depth into `[k/N, (k+1)/N)`, so two scenes on one
page do not depth-test against each other and no depth clear is needed
between them. Cost: `log2(N)` bits of depth precision per scene; with
32-bit float depth and under 16 scenes on a page that is 4 bits of 23,
and a game is one scene with the full range. A page with more scenes
than the slice count allows is refused naming the count (a fixed
number, raised with a measurement, as `particles.md` P3 words it). This
is what lets the frame be one render pass, which tile GPUs want (every
pass boundary stores and reloads the tile; Arm's guidance on bandwidth,
section 11) and which removes the scene's offscreen texture and
composite blend (5.2).

2D content inside a 3D context (a decal's shapes now, HTML labels
later, 7.1): its primitives carry the scene node's 4x4 transform and
are depth-tested in the scene's range with a small depth bias (the
decal's bias from one-scene S7), drawn after the opaque meshes and
before the blended ones. `z-index` inside a scene is refused
(`css-for-3d.md` section 3), so there is no second ordering to
reconcile: depth orders a scene, paint order orders the page.

Opaque reordering. Skia Graphite assigns each draw a z from its
painter's order and lets opaque draws be reordered to cut state
changes, relying on the depth test; clips are depth-only draws
(section 11, Graphite). WebRender draws opaque primitives front to back
with depth test and write and alpha primitives back to front (section
11, WebRender wiki). vapors' list is ordered on the CPU and drawn in
order; the `Order` key is defined so that order-as-depth can be added
for opaque 2D primitives later (slice F13) if a mixed workload shows
pipeline switches in the profile. It is not in the first slices: the
rows page is 52 batches, and every antialiased edge is a blended
fragment, which is the case Graphite's bounds manager exists to order.

### 1.4 Clips and stacking contexts in one list

- A clip is a node in a clip table (2.3) in some transform node's
  space. A primitive under a scroll container is clipped by that
  container's padding box in the container's frame, as `kept_clips.rs`
  splits clips today (inner clip in frame coordinates, the frame's own
  clip on the page; `iu4.md` section 3, "Clips").
- A stacking context changes `place` only, as today; it is not a
  context in 1.3's sense unless it also needs a layer (opacity under 1,
  a filter), when it becomes a layer context.
- A 3D context's primitives are clipped by the scene's viewport
  rectangle (its box, in its frame) and by every clip above that box,
  through the same clip chain; the viewport rectangle is also the
  render pass's scissor for the range, as the composite quad is scissored
  today.
- Hit testing is unchanged in structure: kept hits per frame
  (`kept_hits.rs`), a frame chain translation per hit (`frames.rs:135`),
  a ray through a scene's box into its nodes (`css-for-3d.md` 1.5). A
  transformed element's hit rect is mapped through the node's inverse,
  which is the one new piece (slice F4a).

## 2. Global optimisation over the flat list

### 2.1 What IU4 already made general, and what it keys by shape

IU4 built, for 2D, exactly the mechanisms a flat list needs: work keyed
by change (a batch is rebuilt only when its items change, `iu4.md`
section 2), kept state (`Retained` ranges, atlas pins), coordinates
relative to a frame so a scroll moves no bytes (section 3), and a cull
that is O(log n + shown) (section 4). Three of its structures are keyed
by the shape of today's content:

| Structure | Keyed by | Why it is shape-specific |
|---|---|---|
| `Run` (`batch.rs:23-36`) | six item kinds | a new kind is a new variant and a new painter dispatch arm |
| the frame's shift as a dynamic-offset uniform (`iu4.md` 8.4) | one 2D translation per frame | a transformed element or a 3D node has a matrix, and a batch cannot span frames |
| the scissor per batch | one axis-aligned rectangle | a rounded clip or a clip under a transform cannot be a scissor |

The flat list replaces each with data a shader reads, and keeps the
retention and the cull as they are.

### 2.2 The transform table

A storage buffer of transform nodes, one entry per node the frame
draws under: the page, each scroll frame, each lane row in view
(`splice-layout.md` 5.4 already bounds these to O(visible)), each
transformed element, each 3D node. An entry is a 4x4 `mat4x4<f32>`
(64 bytes) and a class word (4.3); 2D nodes store an affine matrix in
it. Every instance carries a `u32` transform index beside its bytes.
The vertex shader reads `transforms[index]`, multiplies the local
position, and then the view's projection (the viewport for 2D, the
scene camera's view-projection for a 3D context, itself an entry).

Why a table and not the dynamic-offset uniform IU4 chose (`iu4.md`
8.4): IU4 rejected the per-instance index because a batch could not
span frames anyway, since the clip differed per frame. With clips as
data (2.3) that reason goes, and a page of thousands of small scrollers
(section 6, W4) draws its visible rows as one batch per pipeline with
one bind, not one bind per frame. The cost per instance is 4 bytes.
Read-only storage buffers in the vertex stage are core WebGPU (8 per
stage by default, `particles.md` section 0 cites the limits); their
availability on SwiftShader and the Pixel 9 Pro is to be verified by
slice F2's page-contract case before anything depends on it, and a
device without them is refused naming the limit, as every other limit
is.

Writes: an entry is written when its node's matrix changes (a scroll
settles its container's offset: one 64-byte write, where 4c writes one
uniform slot; a 3D node moves: one write, where `slots.rs` writes 80
bytes today). Idle writes nothing. The table is a `Retained` buffer
with the same free-range rules (`retained.rs`), so a frame that stops
drawing a node frees its entry.

### 2.3 Clips as data

A clip table beside the transform table: each entry a rectangle or a
rounded rectangle (`rect`, `radii`) in a transform node's space, and
the index of its parent clip. An instance names its innermost clip; the
fragment shader walks the chain (bounded: the nesting depth of clipping
ancestors, which `clip_tree.rs` already holds; a depth over a fixed
limit is refused naming the element) and multiplies coverage by each
clip's closed-form coverage, through one shared `clip.wgsl` function
that `svg-support.md` V10 also names. The batch's scissor is set to the
bounding rectangle of its instances' clips on the target, so fragment
work outside every clip is still skipped.

What this buys at once: `overflow: hidden` with `border-radius`, refused
document-wide today (`docs/gpu-surface.md`, "Rounded clip on an
ancestor"), draws exactly (a rounded clip's coverage is the same
distance function as a rounded box's edge). What it buys for the flat
list: the clip no longer splits batches, and a clip under a transform
(a rotated scroller, a scene's viewport) is just a rectangle in another
node's space.

Chromium clips rounded rectangles antialiased; the paint rules will
compare a rounded `overflow: hidden` fixture directly (slice F3).

### 2.4 One cull, one sort, one batch

Per frame, over the visible set only, as today:

1. **Cull per context.** A 2D frame's candidates come from its extents
   index (IU4 PR 5), a lane's from `between(view)` (`splice-layout.md`
   5.4). A 3D context's come from an AABB-against-frustum test kept per
   (scene, camera) and re-run only for moved nodes or a moved camera
   (`3d-engine-parity.md` C1, G5). Each cull yields primitive ids; the
   list is never walked whole.
2. **Sort by `Order`.** Candidates are already per context; within a 2D
   frame they are sorted by place as today; within a 3D context the
   blended ones by depth. Nothing is sorted across contexts: the
   contexts' places are in the parent's order.
3. **Batch by state.** Consecutive primitives of one pipeline with the
   same bound textures (the atlas page, an image, a material's textures)
   join one batch. Neither the transform node nor the clip breaks a
   batch. A batch's kept key is its run of primitives as IU4 PR 2b
   matches them (`kept_batches_keys.rs`), unchanged.
4. **Draw.** Per batch: set pipeline (when it changed), bind the
   pipeline's group (transform table, clip table, textures), set the
   scissor to the batch's bound, draw the kept range. The 3D context's
   range additionally sets the viewport depth slice and switches depth
   test on; the pipelines for meshes are made with depth test and write,
   the 2D ones without, in the same pass.

What this replaces: the `Run` enum and the painter's per-kind dispatch
(`painter.rs:134-151` at IU4's reading); `framed_batches.rs`'s split per
frame; the per-frame dynamic-offset bind; the scene's own encoder,
pass, texture and composite quad (`painter_surfaces.rs` keeps serving
application surfaces). What it keeps: `KeptBatches`, `Retained`, the
atlas pins, the extents index, the debug compare of kept bytes against
a fresh build (`painter_kept_check.rs`), which now also compares the
transform and clip indices.

### 2.5 What stays per kind

- Geometry building: a box's instance bytes, a glyph's quad, a mesh's
  vertex buffer. These are per shape family by nature and already
  exist.
- Coverage: per shape family (4.1).
- Text shaping and the atlas: unchanged; the text pipeline keeps its
  own sampler and `adjust()`.
- The 3D material system (`3d-engine-parity.md` 3.1): per material
  kind. A 3D mesh in the flat list is an instance (world matrix and
  colour in the transform table and the instance bytes) of a geometry,
  which is `3d-engine-parity.md` E1 (a), meshes as instances of one,
  decided there as the recommendation; the flat list is where it lands
  (slice F8).

## 3. Compile-time invariants in place of run-time checks

### 3.1 Three forms of proof

The analysable-sources decision makes every source of change visible
to the compiler: classes are toggled by booleans or chosen by enums,
custom properties change through literal-named bindings, `style={expr}`
is refused. `build-time-styles.md` section 1 turns that into two facts
per property per template element: a constant or known-variable
winner, a constant or variable value. This plan uses those facts in
three forms, each with a different run-time shape:

1. **A table of what a page can need (`PaintUses`).** Per crate, as
   `3d-engine-parity.md` G3 proposes `SceneUses` for 3D: which pipelines,
   which transform classes, which clip kinds, which text regimes, whether
   any 3D context, blended material, layer or MSAA target can occur. The
   renderer makes those pipelines at mount (one-scene's F0 row, every
   pipeline before frame 0) and nothing else; an element that at run time
   needs a path its crate's table does not name is refused naming the
   template site and the table. A page whose templates never set
   `transform` never links the unsnapped path. The merged table is the
   union over the crates whose components have mounted, merged on first
   mount as one-scene D3 merges shape tables.
2. **A mark the runtime trusts in release and checks in debug.** The
   splice mark (`build-time-styles.md` section 4) is the first. Its plan
   says the mark decides nothing at run time and the runtime conditions
   stay authoritative. This plan proposes the next step: for a marked
   list, the release build splices without evaluating the conditions,
   and the debug build evaluates them and panics on a contradiction, as
   it does today. The same shape serves a constant transform class (4.3)
   and a constant clip (an element whose `overflow` is constant
   `visible` on it and every template ancestor needs no clip node).
3. **Constants folded into generated calls.** Shape ids (one-scene S5),
   `KeyWinners` (build-time-styles B2), the transform class word of a
   static node, a `{for}` row's lane eligibility (`LaneDecline` reasons
   read from constant `display`, `position`, `overflow` winners rather
   than from used styles at build; `splice-layout.md` section 4).

The rule for choosing between 2 and 3: a fact the runtime would
otherwise compute per update goes in a mark; a fact it would compute
per element goes in the key. A fact that decides whether code exists
goes in the table.

### 3.2 `PaintUses`

Emitted per crate beside `SHAPES` (one-scene 1.3):

```rust
pub struct PaintUses {
    pub transform_classes: u8,   // bit set of TransformClass (4.3)
    pub clip_kinds: u8,          // rect, rounded, coverage
    pub text_regimes: u8,        // snapped, resampled, outline
    pub pipelines: u16,          // bit set over the fixed pipeline list
    pub scenes: bool,            // a 3D context can occur
    pub blended_meshes: bool,
    pub layers: bool,
}
```

From the templates: `transform_classes` from the state of `transform`
on every keyed element (constant `none`: `WholeTranslate` only; a
constant translate with whole px at every scale in `@media` terms is
not provable, since the scale is run-time, so any non-`none` constant
or known-variable transform adds `Affine`; `perspective` or a 3D
function adds `Projective`); `clip_kinds` from `overflow` and
`border-radius` winners together; `text_regimes` from the transform
classes of elements that hold text; `scenes` from `<Scene>` tags;
`pipelines` from the element kinds and paints present. A hand-built
page (tests, `vapors-perf`'s rows page) has no table and gets
everything, counted as `paint_uses_unknown`; the Mount workload, the
racer and one demo page pin it 0.

Loud checks: a debug build records, per frame, the set actually used
and asserts it is within the merged table; a release build refuses at
the primitive that needs an absent path, naming the site. The emitted
table has a snapshot test per demo and the racer.

### 3.3 Examples from vapors' templates

- `crates/racer/ui/hud.wisp`: no `transform`, no `overflow` other than
  the page's, buttons and text. Table: `WholeTranslate`, rect clips,
  snapped text, the quads, shadows and text pipelines, `scenes: true`
  once the world moves into it (one-scene R3a1). The unsnapped path and
  the outline text regime are not built for the racer.
- The Mount workload's `{for}` (`crates/perf/src/mount.rs`): rows with
  `Stacking::default()`; the splice mark proves the list splices and
  the lane applies; the mark is trusted in release (3.1 form 2).
- touche's layer list, when it gets the animated 3D layer view design
  09 names: `transform` with `rotateY` on cards under `perspective`:
  `Projective`, outline text, a layer only where `opacity` applies.
  The table makes those paths exist for touche's page and nowhere else.
- A component with `class:selected={bool}` whose `.selected` rule sets
  `background` only: the toggle is a paint-only change; the primitive's
  colour bytes are rewritten in its kept batch without a re-batch
  (3.1 form 3: the compiler emits which instance attributes a toggle
  can touch; the runtime's `KeptBatches` patch takes a narrower path).
  Not in this plan's slices; named here because the key's shape should
  allow it.

### 3.4 What the proofs replace, structure by structure

| Run-time detection today | Compile-time proof | Run-time remainder |
|---|---|---|
| Splice conditions per update (`order_splice.rs`) | the splice mark (B1's classification over the row template) | debug comparison with the whole walk; unmarked lists keep the check |
| `LaneDecline` from used styles at build (`splice-layout.md` section 4) | the same rules over constant winners per `{for}` | a lane whose eligibility depends on a known-variable property is decided at run time as today, counted |
| Share key and shareability per key per `Kept` (`restyle_share.rs`) | `KeyWinners::shareable` (B3b) | the first-miss verification |
| The painter's per-kind dispatch over a `Run` | `PaintUses::pipelines` fixes the pipeline set | batching by pipeline id |
| `whole_item` checks before a shift (`kept_shift_items.rs:10-19`) | a `WholeTranslate` node's items are whole by construction; the check is a debug assertion on the node | items under `Affine` nodes are never shifted by whole pixels; they are re-transformed |
| A surface's `transform` refusal document-wide (`style/overflow.rs`, `honoured.rs`) | the table names which transform classes exist | none: the refusal is retired where the path is built, kept where it is not |
| Icon subset checks at the update (`svg_checks.rs`, `svg_path_checks.rs`) | one-scene S5a's build-time refusal | a bound `d` (S6) |

The lead's concern was off-path fallbacks: a page that misses a fast
path falls to a slow one in silence. The table and the marks make the
miss visible at build time (`wisp-check` lists each unmarked list, each
declined lane and each path a crate needs, with the rule and
`file:line`), and section 6's workloads make the remaining run-time
misses visible as counts.

## 4. Antialiasing

### 4.1 Per shape family

| Family | Method | Why | Where decided |
|---|---|---|---|
| Rounded rectangles, ellipses, segments, shadows | analytic coverage from the signed distance in the shape's own space, scaled by `fwidth` | exact on a whole-pixel edge, one pixel of antialiasing elsewhere, correct under any affine transform because the distance is evaluated in local space and `fwidth` gives the pixel footprint after the transform; under a projective transform `fwidth` is still the screen-space derivative, so the edge width stays one pixel | built (`quads.wgsl`); the `fwidth` scaling is new (slice F4a) |
| Atlas glyphs | CPU coverage sampled from the atlas, through Skia's mask gamma | matches Chromium's text within 3% ink (`docs/text.md`) | built |
| Tessellated 2D meshes (SVG paths, outline glyphs) | per-vertex outline distance with `fwidth`, or MSAA | one-scene S3b measures both against Chromium on touche's icons | S3b |
| 3D meshes | MSAA, or none | arbitrary triangle meshes have no edge distance; MSAA is the late resolve that works under projection; Graphite uses MSAA where the GPU makes it cheap and avoids it for common 2D primitives with analytic coverage (section 11) | slice F7 measures 4x against none on the UHD 630 and the Pixel 9 Pro |

Two things follow for the one-pass design. First, MSAA is a property
of the render pass: a multisampled colour attachment makes every draw
in the pass multisampled. Analytic 2D coverage is unchanged by that
(the fragment shader runs once per pixel, writes an alpha, and the
blend is the same across the pixel's samples), so a 2D-only page would
pay MSAA's bandwidth for nothing; the pass is therefore multisampled
only when `PaintUses::scenes` (or a mesh pipeline) is set. Second, the
multisampled attachment resolves into the swapchain texture at the end
of the pass (`resolve_target`), with the multisampled colour stored as
`Discard`, which is the form that is near free on tile GPUs (Arm
reports 4x MSAA at a 1 to 2% cost with resolve attachments and
`DONT_CARE` store; section 11). On the UHD 630, which is not a tiler,
the cost is memory bandwidth: 4 samples of 4 bytes of colour and 4 of
depth at 1100 x 720 is about 25 MB written per frame plus the resolve's
reads; at the order of tens of GB/s that is under a millisecond, but
the number is to be measured (F7), not assumed.

### 4.2 The pass's sample count

```
sample_count = if uses.scenes || uses.pipelines has a mesh pipeline { 4 } else { 1 }
```

Fixed per page from the merged table, so no pipeline is made twice and
no frame switches. A 4x page draws its 2D content multisampled too;
the quads, shadows and text pipelines exist in both sample counts in
the pipeline registry, keyed by the page's count. WebGPU guarantees
sample count 4 for the formats in use (`rgba8unorm`, `bgra8unorm`,
`depth32float`), so no device query is needed; a device that cannot
would fail pipeline creation before frame 0, which is a loud failure
at mount, not a blank frame.

Alternatives weighed in decision D2: per-context multisampled targets
resolved and composed (what D1 (a) does today, minus the samples),
which keeps a 2D page single-sampled but costs a pass and a texture
per scene; FXAA as a post pass over 3D ranges (`3d-engine-parity.md`
3.11), which blurs text drawn into the scene.

### 4.3 The transform class and exact snapping

A transform node carries a class:

```rust
pub enum TransformClass {
    /// The composed transform is a translation by whole device pixels.
    WholeTranslate,
    /// Any other affine: a fractional translate, a scale, a rotation, a skew.
    Affine,
    /// A perspective or 3D rotation: w differs from 1.
    Projective,
}
```

The class is of the composed transform from the node to the target:
page, scroll frames, lane rows, transformed ancestors, a scene's
camera. Today every node is `WholeTranslate` by construction: scroll
offsets are rounded (`scroll.rs`), frame shifts are whole
(`frames.rs:23`), and `half_up` commutes with a whole shift
(`paint.rs:121-136`). Lane rows must keep that: a lane child's frame
shift is its `start(handle)` in device pixels rounded `half_up`, not
the f64 sum (a requirement on `splice-layout.md` 5.4 recorded in 7.3).

What the class decides:

| | `WholeTranslate` | `Affine` | `Projective` |
|---|---|---|---|
| Box edges | snapped at paint (`half_up`), as today | unsnapped: the CSS rect times the scale, transformed in the vertex shader; analytic coverage after the transform | the same, with perspective divide |
| Text | regime A: atlas, hinted below 2x, quarter-pixel x, whole-pixel baseline | regime B while the node animates a translation or scale (4.4); regime C under rotation or a settled non-uniform scale | regime C |
| Images | whole-pixel rect, nearest texel at 1:1 | bilinear | bilinear |
| Clips | scissor where the clip is a rect in a `WholeTranslate` node; data otherwise | data | data |
| Hit testing | rect compare | the point through the inverse | the point through the inverse (a singular matrix hits nothing) |

Where the class is decided:

- **Compile time** for a node whose `transform` winner is a constant
  value: `none` gives `WholeTranslate` (given its ancestors are);
  anything else gives `Affine` or `Projective` by its functions. The
  class is folded into the element's key (3.1 form 3) and the crate's
  table (3.2). A keyed element with a constant `none` under constant
  `none` ancestors never evaluates a class at run time.
- **Run time** for a known-variable transform (a toggled class whose
  rule sets `transform`, a bound custom property it reads, a
  transition): the class is computed when the value changes, O(1) per
  change, from the matrix: whole translation if the linear part is the
  identity and the translation's device-pixel parts are whole
  (`fract() == 0.0`, exactly, as `whole_rect` tests), else by `w`.
- **While animating**, the class is held at `Affine` (or `Projective`)
  from the first animated frame to the settle frame, even on a frame
  where the sampled value happens to be whole (4.4). The runtime knows
  an animation is running: a transition or `@keyframes` on the property
  (design 06, when A1 lands) or a frame task writing a bound custom
  property (`css-for-3d.md` 1.6's shape). A bound property written on
  consecutive frames with no declared animation is treated as animating
  and settles after `SETTLE_FRAMES` unchanged frames (a constant,
  declared in code with its reason; 2 is the starting value, measured by
  W7 for the smallest count that shows no mid-motion snap; decision D6).

A class change re-paints the node's subtree once (the items' bytes
change from snapped to unsnapped or back), counted as `parts_reclassed`;
it happens at the start and the end of an animation and never during it.
Everything else about the frame is unchanged: the same kept batches, the
same keys, the transform table entry rewritten per frame while it moves
(64 bytes).

### 4.4 Text: three regimes

Small text is the case that decides whether the page feels right.
Chromium's compositor keeps text sharp at rest and accepts resampled
text during a composited animation; the rules it applies, from the
sources in section 11:

- LCD (subpixel) antialiasing is used only when the raster scale
  matches the transform's scale and the text has an opaque background
  behind it; a transformed element without one loses it (Igalia's
  account of Chromium's raster transforms).
- A composited layer's content is rastered at a fixed scale during a
  transform animation and re-rastered at the ideal scale when the
  animation ends, unless `will-change: transform` holds the raster
  (the blink-dev intent to ship).
- A translation's fractional part is pulled into the raster
  ("subpixel accumulation"), kept through translation-only animations
  and discarded for other transforms (the Chromium and Mozilla changes
  on subpixel accumulation).
- Firefox's rule for an asynchronous transform animation: subpixel
  antialiasing off for its duration, because the transform can become
  non-integer at any moment (the Mozilla bug in section 11).

vapors has no LCD text (grayscale coverage only, `docs/text.md`), so
the question is hinting, grid fitting and resampling, not subpixel
colour. The regimes:

**Regime A, at rest under `WholeTranslate`.** Today's path: glyphs at a
quarter-pixel x step, the baseline on a whole pixel, light vertical
hinting below 2x. Compared with Chromium by the text comparison
(`docs/text.md`, "The comparison page") and the paint fixtures.
Unchanged by this plan.

**Regime B, an animated translation or scale under `Affine`.** The
atlas glyph is drawn at its fractional device position with bilinear
sampling: no quantisation of x to quarter pixels and no rounding of the
baseline while the node moves, so the text slides smoothly and slightly
soft rather than stepping a pixel at a time. For a scale animation, the
glyphs stay the ones rasterised at the start scale (Chromium's fixed
raster during an animation) and are resampled; nothing is re-rasterised
per frame and the hinted stems do not jump between pixel rows. At the
settle frame the node returns to regime A at its final transform: for a
translation that ends on a fraction, the glyphs are re-quantised to the
nearest quarter pixel in x and whole pixel in y, which is what
Chromium's subpixel accumulation yields; for a scale that ends at a new
value, the glyphs are rasterised again at the new ppem (one atlas
request per distinct glyph, counted as `rasterisations`, as the atlas
counts today). A settled scale that is not the device scale (a card
left at `scale(1.37)`) is regime A at that ppem: hinting applies at the
new ppem as it would for a larger font size.

**Regime C, rotation, skew, non-uniform scale, projection.** Glyph
outlines (skrifa, already in the stack) tessellated into the shape
table's mesh form, cached by (font, glyph, tolerance), drawn through the
shapes pipeline with the node's transform, coverage by S3b's method.
The coverage goes through text's `adjust()` with the same gamma and
contrast, so the weight at the switch matches regime A as closely as
coverage-to-alpha can make it: one function answers coverage-to-alpha
for text in both pipelines. No hinting, since there is no grid to fit.
This is the brief's rule ("transformed text uses glyph outlines through
the shape model", `NATIVE_SVG_BRIEF.md`, "Text") and `svg-support.md`
D6 (a). Slug's exact curve coverage (quadratic outlines evaluated per
pixel, resolution independent, public domain since March 2026; section
11) is the alternative for large or 3D text where a flattened mesh's
tolerance would show; it is not the first slice (decision D4).

The switches, and whether they are visible:

| Switch | When | What changes on screen | Mitigation |
|---|---|---|---|
| A to B | the first animated frame | edges go from exact to up to half a pixel soft at the same place | none needed: motion starts on this frame; Chromium's first composited frame is the same resample |
| B to A | the settle frame | a sub-pixel pop: the text re-quantises and, after a scale, re-rasterises hinted | Chromium does the same (re-raster at animation end); decision D5 asks whether to cross-fade over two frames; recommended no, so the rest state is the oracle's |
| A to C, B to C | a rotation or projection starts | coverage changes from atlas to outline: weight may differ by what `adjust()` cannot equalise | measured by slice F6 at rest in both regimes against Chromium; the threshold where the difference passes the paint rules is recorded |
| C to A | a rotation ends at upright | as above, reversed | the same measurement |

### 4.5 Where the decision is made, and the oracle

- Static: the compiler, per template element (4.3); the class is a
  constant in the key and the crate's table.
- Dynamic: the elements layer, at the change of a known-variable
  transform and at the start and settle of an animation, per node, not
  per primitive. The painter reads the class from the transform table
  entry's word; shaders never decide.
- Never in the fragment shader by inspecting the matrix: the decision
  would be per fragment, per frame, and invisible to the counts.

Oracle policy for transformed content (new; today's fixtures have no
transforms): Chromium is the oracle at rest, before an animation starts
and after it settles, through the existing paint rules at 1x and 2x.
During an animation, Chromium's frames are resampled rasters of a
layer, which is not a pixel-exact reference; the mid-animation check
is against an exact-coverage reference render (the shape drawn at 8x
and box-filtered, as `paint_support/exact.rs` declares exact pixels
today), with a rule that no frame moves a glyph's centroid by more than
the transform moved it plus a stated tolerance, which is what catches a
snap. Scenes keep design 09's rule: reference renders on lavapipe,
since no browser draws a wgpu scene.

## 5. Costs

Measured figures are cited; everything else says how it will be
measured. The method is the finish line's: release build, UHD 630,
load under 1.5, three runs, medians, instruction counts for CPU phases
and `GpuTimer` for GPU time, written to `docs/perf/`.

### 5.1 What the racer pays today for two renderers

From `docs/perf/scene-run-2026-11-13.md` (today's `Track` renderer,
1100 x 720): GPU time median 5.1 ms, CPU frame 2.5 ms, frame-to-idle
7.2 ms, 25 draw calls, 33 KB of buffer bytes per frame. The scene's
backing is an `Rgba8Unorm` texture of 1100 x 720 x 4 = 3.17 MB and a
`Depth32Float` of the same size; each frame clears both, draws the
scene, stores the colour, then the painter's pass blends the texture
over the page as one full-content-box quad. Under the flat list the
scene draws into the page's pass: the backing texture, its clear and
store and the composite blend go (about 3.17 MB written and read once
each per frame less); the depth attachment moves to the page's pass at
the same size. With MSAA 4x on (4.2) the multisampled colour and depth
add about 25 MB of writes per frame and a resolve. Net GPU time: to be
measured in F8 (one pass, no MSAA) and F7 (MSAA) against this baseline.
The expected direction is a small saving without MSAA and a cost with
it that the number decides.

### 5.2 CPU side

- Batching: the rows page is 52 batches for 17 rows; the flat batcher
  joins batches that differ only by frame or clip, so a page of one
  scroller is unchanged and a page of many scrollers has fewer. Measured
  by the `batches` draw step (`iu4.md` PR 0's spans) on W1 and W4.
- Per instance: 4 bytes for the transform index and 4 for the clip
  index, so a quad grows from 128 to 136 bytes (6%); glyph quads from
  192 to 200. Memory for 100 visible rows stays about 0.5 MB (`iu4.md`
  section 2's estimate).
- Transform table writes: one 64-byte write per moved node per frame.
  A scroll writes 64 bytes where 4c writes one uniform slot; an
  animated card writes 64 bytes per frame. Idle 0. Pinned per workload.
- Reclassing: a transition's start and end repaint the node's subtree
  once each: O(subtree) twice per animation, counted
  `parts_reclassed`; never per frame.
- The sort and cull are unchanged in order: O(log n + shown) per 2D
  frame (IU4 PR 5), O(nodes) per 3D context on a camera move and O(1)
  per moved node (`3d-engine-parity.md` G5).

### 5.3 Build and binary

- `PaintUses` removes pipelines a page cannot need; the pipeline
  registry's count is pinned per demo. Binary size is measured by the
  existing method (`cargo xtask web`, brotli bytes, as
  `complex-script-breaks` was measured) in each slice that adds a
  shader; the clip and transform tables add two small WGSL functions
  shared by every pipeline.
- The outline text regime pulls the tessellator into the runtime for
  crates whose table names it, behind `svg-support.md`'s
  `runtime-shapes` feature; a page without transformed text ships none
  of it.

### 5.4 What is not known until measured

MSAA 4x on the UHD 630 (F7); storage-buffer reads in the vertex stage
on SwiftShader and the Pixel 9 Pro (F2); the visibility of the B to A
pop on 13 px text (F5's screenshots at the settle frame, judged by eye
once and by the centroid rule thereafter); the A/C weight difference
(F6); the cost of the clip chain walk per fragment at nesting depth 3
to 5 (F3, `GpuTimer` on the rows page with a rounded scroller).

## 6. Generality: off-path workloads for `vapors-perf`

`vapors-perf` measures the rows page (block rows; a flex-column list
for `FlexText`), its Insert, Resize, Scroll and ScrollSize changes, and
the racer (`crates/perf/src/workloads.rs:52-71`). Every structure the
lead named as shape-specific (the layout lane for block and flex lists,
the extents index for vertical scrolling of short steps, the splice
conditions, the share keys) is exercised only by pages those workloads
have. A workload per shape the structures do not cover, each with the
count that shows a miss:

| # | Workload | Page | Change per frame | What it exercises | Judged by | Count that shows special-casing |
|---|---|---|---|---|---|---|
| W1 | `NestedScroll` | a scroller of 100 scrollers, each of 100 rows | the outer scrolled one row; every tenth frame an inner one | frames in frames: `Frames::of` per paint, the index per frame, the clip chain | `--scaling` over 10, 100, 1000 inner scrollers: `paint` and `draw` flat | `frames_resolved` per update (should be O(visible frames)); `batches` count |
| W2 | `WrapFlex` | a `flex-wrap: wrap` gallery of 3,000 tiles | the container's width alternates by 1 px, so rows re-wrap | a layout the lane declines (`splice-layout.md` section 4) | reported, judged after a wrap lane exists | `lanes_declined` with reason `Wrap`; `boxes_laid_out` |
| W3 | `Grid` | a CSS grid of 50 x 60 cells (the `grid` feature) | one cell's text | grid is declined by the lane; the extents index with items whose paint order is row-major | `--scaling` over 1,000, 3,000, 10,000 cells: every phase flat for an edit | `lanes_declined` reason `Grid` |
| W4 | `SmallScrollers` | 3,000 code blocks, each `overflow: auto`, in a page scroller | the page scrolled one block | `iu4.md` PR 5's open M2: every frame searched on every sort, O(frames · log steps) | `--scaling` over 300, 1,000, 3,000 blocks: `draw` flat | `frames_searched` per draw (should be the frames whose clip is a candidate) |
| W5 | `MixedText` | a transcript: paragraphs of 20 to 400 words, three families, bold and code spans, 3,000 paragraphs | one paragraph's text | text runs of many sizes and ink extents: the tall-step list in the index; the atlas under many glyphs; the text cache by content (T1) | `--scaling`: `layout` and `draw` flat | `tall_steps_tested` per draw; `rasterisations` per frame after warmup (0) |
| W6 | `Positioned` | the rows page with every 7th row `position: relative; z-index: 1` and a fixed footer | a Class edit on a positioned row | paint order that is not tree order: the splice declines (`FreedStacked`), candidates sorted by place | `--scaling`: an edit flat; Insert reported | `splices_declined` reason; `order_walks` |
| W7 | `Translate` | a card of text and a button under `transform: translate(x, 0)` | x advances 0.37 px per frame for 60 frames, rests 10, repeats | the transform class at run time, regime B, the settle, the table write | pinned: `parts_reclassed` 2 per cycle, `uniform_writes` or table writes 1 per moving frame, `batches_written` 0 while moving; the centroid rule (4.5) | `parts_reclassed`, `rasterisations` (0 while moving) |
| W8 | `ScaleAnim` | the same card under `scale(s)` | s from 1.0 to 1.37 over 60 frames, rests, back | regime B under scale, re-raster at settle only | pinned: `rasterisations` 0 while moving, one set at each settle | `rasterisations`, `parts_reclassed` |
| W9 | `Icons` | 1,000 of touche's ten icons in a grid | one icon's colour class | the shape table and the shapes pipeline (one-scene S3a1 onward); before them, the icon route | `--scaling` over 100, 1,000, 10,000 icons: an edit flat; mount time per icon reported | `shapes_tessellated` 0 per frame; `shape_bytes_uploaded` 0 idle |
| W10 | `RoundedClips` | the rows page in a scroller with `border-radius: 12px; overflow: hidden`, nested in a rounded card | Scroll | clips as data (2.3): the chain walk, the scissor bound | Scroll's `draw` equal to the plain scroller's within noise; `GpuTimer` difference recorded | `clip_depth_max` |
| W11 | `SceneWithHtml` | the one-scene racer page with 50 HTML labels | the racer's scripted driver | the one-pass design: 3D range, depth slice, HTML over it; later labels inside the scene (F9) | one-scene P0's judge: no frame over 16.7 ms | `passes` per frame (1), `scene_contexts`, `depth_slices` |
| W12 | `MixedPage` | a documentation page: a sidebar scroller, a main scroller of mixed text with icons and images, a sticky header, a rounded modal with a shadow, one small scene | Scroll of the main scroller; a Class edit | everything at once: whether the batcher, the index and the tables hold together on an ordinary page | `--scaling` over 1x, 3x, 10x content: edit and scroll flat | the union of the counts above |

Rules for the rows: each is one PR under 400 lines including its page,
built from the elements tests' page builders as `rows_list.rs` is, so
the harness times what the tests check; each ships with its first
numbers in `docs/perf/` and a sentence saying which counts a later slice
must move; a workload whose page a feature refuses today (W3 without
`grid`, W10 before F3, W11 before F8) is added with the slice that
lifts the refusal, so the harness never carries a page that cannot
mount. W7 and W8 need a transform that moves; before CSS transitions
exist (design 06 A1) they drive a bound custom property from the
workload, which is the form 4.3's run-time rule covers.

## 7. Relation to the existing plans

### 7.1 `one-scene.md` (property 9)

- **Keep:** D1 (a) for property 9 as decided; every S row (the shape
  table is the first brick of this plan's `Mesh` shape); P0 and P1; R1
  to R3b; D4 (d), D5 (b), D6, D7.
- **Change:** S7 (`SurfaceFrame::shapes`, shapes drawn by
  `SceneRenderer` from the store with a node's world matrix) and S8's
  `<Decal>` rendering path. Under the flat list a decal's shapes are
  primitives with the node's 4x4 transform in the scene's context
  (1.3), drawn by the shapes pipeline with depth test on, so S7's
  renderer-side route is not built; S8's component and its CSS stay.
  If the finish line's schedule reaches S7 before this plan starts, S7
  is built as planned and retired by F9; the choice is decision D9.
- **Drop, later:** `Item::Surface` as the scene's host, once F8 lands;
  the surface element stays for application renderers.

### 7.2 `iu4.md`

- **Keep:** retention (PRs 1 to 3), frame coordinates (4-i, 4-ii), the
  extents index (PR 5), every debug check.
- **Change:** 4c's dynamic-offset shift becomes an entry in the
  transform table (2.2), which revisits 8.4 with a new reason (batches
  can span frames once clips are data); "consecutive items share a
  batch only within one frame" (section 3, "Batching") is lifted;
  `Painted::frame_draw` hands the painter an index, not a shift.
- The index's incremental update required by the lane (PR 5, "Requirement
  for the layout lane") is unchanged and needed here too.

### 7.3 `splice.md` and `splice-layout.md`

- **Keep:** everything.
- **Requirement added:** a lane child's frame shift is `half_up` of its
  start in device pixels (4.3), so `WholeTranslate` holds for lane rows
  and the kept-paint rounding argument (`iu4.md` section 3) still
  commutes; the f64 sum is for layout, the rounded value for paint.
  `splice-layout.md` 2.1's tolerance note stays for the layout check.
- **Later:** the splice mark trusted in release (3.1 form 2) after
  build-time-styles S1 lands and W6 shows the check's cost, if it does.

### 7.4 `build-time-styles.md`

- **Keep:** U1, U2, T1, R1a, R1b, B0 to B3b, S1, M1, M2.
- **Extend:** B1's classification reaches `transform`,
  `transform-origin`, `perspective`, `overflow-*`, `border-radius` and
  `clip-path` for the transform and clip classes; B2 emits the class
  word in the key; a new slice emits `PaintUses` (F10 here).

### 7.5 `css-for-3d.md`

- **Keep:** the property set, D2 (`transform` refused on scene nodes:
  the node's matrix is one transform table entry and the prop is its
  one writer), D3, D4, the hit test (C4).
- **Change:** C5's blended pass becomes the blended range of the
  scene's context in the one pass (1.3); the sort is the same. C9
  (`backface-visibility`) is a pipeline variant in the registry.

### 7.6 `svg-support.md`

- **Keep:** V1 to V9, V13 to V23, D1 (flatten at build time), D4, D5,
  D7, D8, D9.
- **Change:** V10's clip descriptor "in a per-draw uniform that every
  pipeline reads" becomes the clip table (2.3), indexed per instance;
  V10 is where the table is built if it comes first, F3 otherwise.
  D2 (clips by coverage, not stencil or MSAA): if F7 turns MSAA on for
  pages with meshes, a stencil clip becomes viable on those pages; the
  recommendation stays coverage, since the clip table serves every page
  and stencil would serve only the multisampled ones. V11's layer
  mechanism is the layer context of 1.1. D6 (text: atlas upright,
  outlines transformed) is 4.4's regimes A and C; 4.4 adds regime B,
  which D6 did not have.

### 7.7 `3d-engine-parity.md`

- **Keep:** the rows and order.
- **Change:** E1 (a) (meshes as instances of one, `slots.rs` retired)
  lands in F8 as part of putting meshes in the flat list; M3's
  `SceneUses` is one group of fields of `PaintUses` (3.2); A1's blended
  list is the context's blended range; the culling of C1 feeds the
  flat cull (2.4); Q1's FXAA is not needed for 3D edges once F7 decides
  MSAA, and stays an option if F7 decides against it.

### 7.8 `particles.md`, `threading.md`

- Particles: the compute pass stays a pass before the frame's render
  pass; the billboards draw in the scene's blended range, after the
  meshes, as planned. P6's guards are unchanged.
- Threading: TH5 (a render thread) gets simpler, since draw reads one
  list and two tables; nothing else changes. Decision 4 there stands.

### 7.9 `docs/design/09-3d.md` (stage 1: CSS 3D transforms on elements)

- **Change:** "an element with a 3D transform becomes a layer: its
  subtree is painted to a texture ... composited as a textured quad"
  and "text inside a transformed layer is rasterised at the layer's
  scale ... and resampled" are replaced. Under this plan a transformed
  element is a transform node; its primitives carry the matrix and are
  drawn directly with late antialiasing; text follows 4.4. A layer is
  made only by the grouping properties that need compositing (`opacity`
  under 1, `filter`, `mask`, `mix-blend-mode`), as 1.1 says. This is
  the one place this plan departs from an agreed design, and it is why
  Tom's direction on blur ("never feels blurry needlessly") is met:
  Chromium's layer model resamples a raster; the flat list re-covers
  the geometry.
- **Keep:** stage 1's semantics (CSS Transforms 2 as the oracle,
  flattening by grouping properties, hit testing through the inverse),
  stage 2's typed scene components (now `3d-2.md`), the oracles table.

## 8. Slices

Each under about 400 lines with tests (estimates), built by a fresh
agent, each check proven by breaking what it guards and the commit
saying so. Every slice keeps every existing Chromium comparison and
`gpu_*` test unchanged unless its row says which and why. Each ships
its numbers in `docs/perf/`. Ordered so each is useful on its own:
F1 to F3 improve today's 2D renderer and lift a refusal before any 3D
content enters the list.

| # | Slice | Content | Accepted when | Depends on |
|---|---|---|---|---|
| F1 | Transform class on frames | `TransformClass` in `vapors-scene`; each frame (`frames.rs`) carries `WholeTranslate`; paint takes `snap` from its frame's class (always true at this head); a debug assertion that every item under a `WholeTranslate` frame is whole (`whole_item`); `FrameCounts::parts_reclassed` (0) | Pixels unchanged on every fixture; the assertion proven by breaking (a fractional shift panics naming the frame); W6 (`Positioned`) added with its first numbers | none |
| F2 | The transform table | A `Retained` storage buffer of 4x4 entries; a `u32` index per instance in the quad, shadow, text and image layouts; the shaders read the table; `Shift` and the dynamic-offset slots retired for the painter (touche's immediate path untouched); a page-contract case that reads the table on SwiftShader; the Pixel 9 Pro checked by hand and recorded in `docs/web.md` | Scroll at 3,000 rows: writes equal to 4c's (one entry per scrolled frame); Idle 0; `kept_bytes` debug compare extended to the index; pixels unchanged; a device without vertex-stage storage buffers refused naming the limit (unit test with a faked limit) | F1 |
| F3 | Clips as data | The clip table (rect and rounded rect, parent index); a `u32` clip index per instance; `clip.wgsl` shared by every pipeline; the scissor as the batch's bound; batches joined across frames and clips; `overflow: hidden` with `border-radius` and a rounded clip on a `gpu-surface` ancestor no longer refused | A Chromium fixture of a rounded scroller holding text, an image and a shadow at 1x and 2x passes the paint rules; W10 added; W1 and W4 added with before/after batch counts; the clip-depth refusal proven by breaking | F2 |
| F4a | Boxes under an affine transform | CSS `transform: translate | scale` on elements accepted (the `Deferred` entry retired for those functions); a transformed element is a transform node with class from its matrix; items built unsnapped under `Affine`; `fwidth` scaling in `quads.wgsl` and `shadows.wgsl`; hit testing through the inverse; text under an `Affine` node refused naming F5; a static transform's class folded into the key (3.1 form 3) | Chromium fixtures at rest: a card at `translate(10.5px, 3.25px)` and at `scale(1.37)` pass the paint rules at 1x and 2x; `elementFromPoint` agrees on a click through the transform (events oracle); W7's page mounts (text replaced by boxes until F5) | F3; build-time-styles B1 for the key's class |
| F4b | Animating transforms | A bound custom property read by `transform` as the run-time source; the animating state (4.3): class held through the motion, `SETTLE_FRAMES` declared with its reason, `parts_reclassed` at start and settle only; the table entry rewritten per frame | W7 (boxes): `parts_reclassed` 2 per cycle, `batches_written` 0 while moving, table writes 1 per moving frame; the centroid rule against an exact-coverage reference; a snap mid-motion proven to fail it by breaking (class recomputed per frame) | F4a |
| F5 | Text under translation (regime B) | Atlas glyphs at fractional positions with bilinear sampling while the node animates; re-quantised at settle; the baseline not rounded while moving | W7 with text: `rasterisations` 0 while moving; the settle frame equals regime A's pixels for that transform (bytes compared); screenshots at the settle frame recorded; the Chromium comparison at rest before and after unchanged | F4b |
| F6 | Text under rotation (regime C) | Glyph outlines tessellated into shape entries cached by (font, glyph, tolerance), behind `runtime-shapes`; drawn through the shapes pipeline with `adjust()`; `rotate()` and `skew()` accepted; the A/C weight difference measured at rest at 13, 16 and 28 px against Chromium at 0, 15, 45 and 90 degrees | The difference recorded with the paint rules' verdict per size; the switch rule's thresholds in `docs/text.md`; a rotated-text fixture passes or the question returns to Tom with the numbers | F5; one-scene S2, S3a2, S3b |
| F7 | MSAA decision | The pass sample count from a page flag (the `PaintUses` field before F10 exists); every 2D pipeline in both counts in the registry; a resolve into the target; `GpuTimer` A/B on the rows page, W10 and the racer page at 1 and 4 samples on the UHD 630 and the Pixel 9 Pro; a mesh fixture (a tilted cube's edge) compared with an 8x reference at 1 and 4 | The numbers in `docs/perf/`; decision D2 answered with them; pixels unchanged at 1 sample | F3 |
| F8 | The scene in the one pass | `Item::Scene` as a context: the scene's range drawn in the painter's pass with the depth attachment (present only when a scene is on the page), the viewport depth slice per scene, the slice-count refusal; meshes as instances of one in `Retained` with the world matrix in the transform table (E1 (a)); `slots.rs` retired; the scene's offscreen texture and composite gone; `Item::Surface` kept for application renderers | The ground and object check (`3d-2.md`) passes in the one pass; two scenes on one page do not depth-test each other (pixel test); HTML over the scene unchanged; the racer's 60 s run (one-scene P0's judge) before and after in `docs/perf/`; `passes` per frame pinned 1; `FrameCounts::surfaces` for the scene's work folded into the frame's counts with the pins moved, each proven by breaking | F2, F7; one-scene R3a2 (the racer on `SceneRenderer`) |
| F9 | Primitives under a scene node | Shapes (decal paths) as primitives with a 4x4 transform in the scene's context, depth bias, drawn after opaque meshes; `<Decal>` from one-scene S8 on this route; S7's `SurfaceFrame::shapes` not built or retired | One-scene S7's acceptance on this route (a decal on a plane at 60 degrees against an 8x reference; no depth fighting at the racer's widest zoom); a livery toggle writes paint bytes, no geometry | F8; one-scene S5b |
| F10 | `PaintUses` | The compiler emits the table per crate from B1's classification over `transform`, `overflow-*`, `border-radius`, scene tags and element kinds; the registry makes pipelines from the merged table at mount; the unsnapped path, outline regime and clip kinds are built only when named; `paint_uses_unknown` for hand-built pages; `wisp-check --paint` prints the table and each element that widens it with `file:line` | Snapshot per demo and the racer; a `<Mesh>` added from Rust on a page whose crates name no scene refused naming the site; `paint_uses_unknown` 0 pinned on Mount, the racer and one demo; the debug within-table assertion proven by breaking | F4a, F8; build-time-styles B3b |
| F11 | Projective transforms | `perspective`, `rotateX/Y/Z`, `rotate3d`, `matrix3d`, `transform-style: preserve-3d` (no plane splitting, as Chromium), `backface-visibility`, `transform-origin` with z: the `Projective` class; text in regime C; hit testing through the inverse with a singular matrix hitting nothing | WPT `css/css-transforms/` reftests named in design 09's oracle table run as pixel comparisons at rest; touche's layer-view card fixture passes; `getBoundingClientRect` agreement on the projected box | F6, F4b |
| F12 | HTML inside a scene | A scene node hosting an HTML subtree laid out in CSS px in a plane: its primitives in the scene's context with the node's matrix, regime C text, hit testing by the scene's ray then the plane's inverse | A label on the racer's craft follows it through a turn (pixel test); a click on the label reaches its listener; the 60 s run still passes | F9, F11 |
| F13 | Opaque reordering | Order-as-depth for opaque 2D primitives (Graphite's and WebRender's rule) if W12's profile shows pipeline switches over a stated share of draw; opaque front to back with depth write, blended back to front | Only on that measurement; `draw` on W12 down by the predicted share; pixels unchanged | F8, W12 |
| W1 to W12 | Workloads | As section 6; one PR each, placed with the slice that lets the page mount | Each with first numbers in `docs/perf/` | per row |

Critical path: F1, F2, F3 (useful alone: fewer binds, rounded clips),
then F4a, F4b, F5 (transforms on HTML with crisp text at rest), then
F7 and F8 (one pass with the scene), then F6, F9, F10, F11, F12. F13
only on a number. Workloads W6 (F1), W1, W4, W10 (F3), W7 (F4a), W8
(F5), W9 (one-scene S3a1), W11 (F8), W2, W3, W5, W12 (any time after
F1) carry the measurements.

## 9. Decisions for Tom

**Decided 2026-11-13 (Tom): D1 to D10 all as recommended.** D1 (a) one flat list after property 9; D2 (a) MSAA 4x on pages with a mesh pipeline, subject to F7's number; D3 (a) Chromium's rule; D4 (a) flattened outlines first; D5 (a) snap on settle; D6 (a) detect from consecutive-frame changes, (c) if it chooses wrong; D7 (a) clip table; D8 (a) storage-buffer table with the per-node fallback, reported; D9 (a) build S7 now; D10 (a) after this iteration, workload rows earlier. Separately, S2's shape antialiasing extrudes the outline by half a pixel (Tom, 2026-11-13, #306).

Each with options, the trade-off and a recommendation. Sections 1 to
8 follow the recommendation; where another option changes a row, the
decision says which.

**D1. The flat list (one-scene D1 (b)) as the direction after property
9, with D1 (a) kept for the finish line.** (a) Yes: the S rows and
IU4's retention are shared bricks; F1 to F3 are improvements to today's
2D renderer whatever follows; F8 replaces the surface composite for
scenes once property 9 is measured. (b) Stay with D1 (a) and build
one-scene S7 (shapes drawn by `SceneRenderer`) as the long-term route:
two renderers, HTML never inside a scene, two antialiasing policies.
Recommend (a).

**D2. Late antialiasing for meshes.** (a) MSAA 4x for the whole pass on
pages whose table names a mesh pipeline, single-sampled otherwise; 2D
coverage stays analytic (4.2). (b) Per-scene multisampled targets
resolved and composed: a 2D page is untouched, each scene costs a pass
and a texture, HTML inside a scene is composited not drawn. (c) No MSAA;
FXAA over scene ranges later (`3d-engine-parity.md` Q1): blurs text and
decals in the scene. Recommend (a), subject to F7's number on the UHD
630; if 4x costs over a stated share of the frame there (a tenth of
16.7 ms, say), (b) for desktop and (a) on tilers, chosen per adapter and
reported.

**D3. Text under a scale animation.** (a) Chromium's rule: the glyphs
rasterised at the start scale are resampled during the animation and
re-rasterised hinted at settle (regime B). (b) Outlines through the
shape pipeline for the duration (regime C): crisp at every frame, a
weight change at both ends. (c) Re-rasterise at each frame's ppem:
churn in the atlas and hinted stems jumping rows. Recommend (a): it is
what the oracle does and the only one with no visible change at the
start.

**D4. Text under rotation and projection.** (a) Flattened outline
meshes through the shape pipeline with S3b's coverage, `adjust()`
shared (regime C). (b) Slug-style exact curve coverage per pixel
(public domain since March 2026): resolution independent, no flattening
tolerance, a per-pixel loop over the glyph's curve bands; a second text
shader. Recommend (a) first; (b) as a measured later row for large or
3D text if F6's flattening tolerance shows at the sizes touche's layer
view uses.

**D5. The settle pop.** (a) Switch from regime B to A on the settle
frame with no cross-fade: the rest state is pixel-identical to a page
that never moved, which is the oracle's state. (b) Cross-fade over two
frames: no pop, two frames of double-drawn text, a rest state the
oracle cannot check until the fade ends. Recommend (a).

**D6. Detecting motion from a bound property.** (a) A declared
animation (transition, `@keyframes`, a frame task) switches regimes; a
bound custom property changing on consecutive frames is treated as
animating and settles after `SETTLE_FRAMES` unchanged frames, the
constant declared with its reason and measured by W7. (b) Declared
animations only; a per-frame signal write snaps each frame and
`wisp-check` warns that it will jitter. (c) A template marker on the
binding (`transform={expr} animated`) so the compiler knows. Recommend
(a) now and (c) if a page shows the heuristic choosing wrong; (b) makes
the common signal-driven case wrong by default.

**D7. Clips as data (2.3).** (a) A clip table read per fragment, the
scissor as the batch's bound. (b) Scissor for rects and a stencil
buffer for rounded and transformed clips: no per-fragment chain walk,
but stencil state splits batches and needs the stencil attachment on
every page. (c) Graphite's depth-only clip draws: needs order-as-depth
(F13) first. Recommend (a); F3 measures the chain walk's cost on W10.

**D8. The transform's transport.** (a) A storage-buffer table indexed
per instance (2.2). (b) IU4's dynamic-offset uniform per node: no
vertex-stage storage reads, one bind per node, batches cannot span
nodes. Recommend (a), after F2's page-contract case verifies the reads
on SwiftShader and the Pixel 9 Pro; if either lacks them, (b) stays on
the web and (a) natively, both behind one trait, reported in the status
line.

**D9. One-scene S7 before or after this plan.** (a) Build S7 as
planned if property 9 reaches it first; F9 retires it. (b) Hold S7 and
reach property 9's decal through F8 and F9: property 9 waits for F1 to
F8. Recommend (a): property 9 is the finish line and S7 is about 400
lines; the retirement is a deletion.

**D10. Placement.** (a) After the finish line, in section 8's order,
with the workload rows W1 to W6 and W9 allowed earlier since they add
measurements only and touch no code on the finish line's path. (b)
Everything after the finish line, workloads included. (c) F1 to F3
during the finish line's tail as 2D improvements. Recommend (a); (c)
is tempting because F3 lifts a refusal touche may want, but F2 changes
every pipeline's vertex layout, which the IU4 and one-scene rows are
still landing on.

Implementation choices recorded here, not for Tom: the depth slice per
scene through the viewport's depth range rather than a clear per scene
(1.3); 4x4 entries for 2D nodes too, so one shader path (2.2); the
class word in the transform entry, so the painter reads one table
(4.3); regime B's bilinear sampling rather than a re-raster per
fractional step (4.4); the exact-coverage reference for frames in
motion (4.5).

## 10. Risks

1. **MSAA on the UHD 630.** The whole pass goes multisampled on pages
   with meshes; the cost there is a bandwidth estimate (4.1) until F7
   measures it. D2's fallback is per adapter.
2. **Vertex-stage storage buffers on the web.** Core WebGPU has them;
   SwiftShader and the Pixel 9 Pro are unverified. F2 verifies before
   anything depends on them; D8's fallback keeps dynamic offsets there.
3. **Depth-range slicing.** Two scenes in one pass share one depth
   buffer through viewport ranges; a driver that clamps or quantises
   `min_depth`/`max_depth` differently would let scenes bleed. F8's
   two-scene pixel test runs on lavapipe, ANV and SwiftShader.
4. **The regime switches are visible.** B to A is a sub-pixel pop
   Chromium also has; A to C is a weight change `adjust()` may not
   equalise. F5 and F6 measure; if F6's difference fails the paint
   rules at body sizes, rotated body text is refused naming the size
   until D4 (b) is built, never drawn soft.
5. **No oracle in motion.** Chromium's mid-animation frames are
   resampled layers. The exact-coverage reference and the centroid rule
   (4.5) are this plan's, not a browser's; a reviewer should read them
   as a rule vapors sets for itself.
6. **Chromium's rasteriser is changing under the oracle.** Graphite
   shipped on Apple-silicon Macs in July 2025 (section 11) and its
   antialiasing and ordering differ from Ganesh's; the Linux oracle is
   Chromium 155 as installed. A Chromium update that moves a pixel
   fails `--check`, which is the intended loud failure; the fixtures
   are re-read, not relaxed.
7. **Per-instance indices grow every layout.** 8 bytes per instance
   (5.2) and a vertex layout change in every pipeline; touche's
   immediate path (`vapors_canvas`, `iu4.md` section 6) keeps its
   signatures, so a pinned touche builds unchanged, but a touche pin
   bump follows F2.
8. **The clip chain walk per fragment.** Depth 3 to 5 on an ordinary
   page; each level is a rounded-rect distance. F3 measures on W10; a
   depth over the fixed limit is refused, never approximated by
   dropping a clip.
9. **Opaque reordering with antialiased edges.** Every edge is a
   blended fragment, so order-as-depth (F13) needs Graphite's
   bounds-manager reasoning to be exact; it is gated on a number and
   may never be built.
10. **Scope.** Fourteen slices after a finish line that is itself
    open. F1 to F3 are worth building alone; nothing past F3 is a
    prerequisite for the rows before it; stopping after any slice
    leaves pixels unchanged on every fixture.
11. **Lane rows at fractional starts.** 7.3's requirement (rounded
    frame shifts) must land in splice-layout PR 4, or lane pages under
    a fractional scale differ from Chromium by a pixel at row edges and
    the `WholeTranslate` assertion of F1 fails on them; F1 lists the
    assertion as the check that would catch it.

## 11. Sources

Read on 2026-11-13 through web search summaries; each is cited where
used. Where a summary did not settle a point the text says "not
verified".

- Chromium raster transforms and LCD text (LCD only when the raster
  scale matches the transform's scale and the text has an opaque
  background): [Igalia, "Improved raster transforms with non-uniform
  scales" (2021)](https://blogs.igalia.com/obrufau/2021/07/20/improved-raster-transforms-with-non-uniform-scales.html).
- Re-raster at the ideal scale when a composited layer's scale changes,
  unless `will-change: transform`: [blink-dev, "Intent to ship:
  re-rastering composited layers on scale change"](https://groups.google.com/a/chromium.org/g/blink-dev/c/Ufwrhx_iZ7U).
- Raster scale held during a scale animation; the 2014 GPU-raster text
  exception and its revert: [Chromium code review 422233008](https://codereview.chromium.org/422233008),
  [272283003](https://codereview.chromium.org/272283003); a third-party
  reading of `picture_layer_impl.cc`'s current conditions (not verified
  against source): [grida.co, "Resolution scaling during interaction"](https://canary.grida.co/docs/wg/research/chromium/resolution-scaling-during-interaction).
- Subpixel accumulation pulled into the raster for translations, kept
  through translation-only animations, discarded for other transforms:
  [Chromium code review 1848613002](https://codereview.chromium.org/1848613002),
  [Chromium commit 570f87d](https://chromium.googlesource.com/chromium/src/+/570f87d82f2588a125f9a19f0430989ffb7db05d%5E%21),
  [Mozilla bug 1681364 (WPT translation-animation-subpixel-offset)](https://bugzilla.mozilla.org/show_bug.cgi?id=1681364),
  [the WPT test](https://searchfox.org/firefox-main/source/testing/web-platform/tests/css/css-animations/translation-animation-subpixel-offset.html).
- An asynchronous transform animation disables subpixel antialiasing
  for its duration (Firefox): [Mozilla bug 1340206](https://bugzilla.mozilla.org/show_bug.cgi?id=1340206).
- Chromium's initial raster translation so text is sharp under a
  fractional initial transform: [Chromium commit d283a46](https://chromium.googlesource.com/chromium/src/+/d283a465cd615caa51d758a222cf6b2d43f1514c).
- Skia Graphite: order as depth, clips as depth-only draws, greedy draw
  sorting, analytic coverage for rounded rectangles to avoid MSAA, MSAA
  where cheap with a CPU-atlas fallback, launched on Apple-silicon Macs
  in July 2025: [Graphite `DrawPass.cpp`](https://skia.googlesource.com/skia/+/c005895e76627fd8662f655ba9448e43aa75955d/experimental/graphite/src/DrawPass.cpp),
  [`AnalyticRRectRenderStep.cpp`](https://skia.googlesource.com/skia/+/f7ff1808d82c4807788ded2a2260152d7596a4d1/src/gpu/graphite/render/AnalyticRRectRenderStep.cpp),
  [Chromium blog, "Introducing Skia Graphite"](https://blog.google/chromium/introducing-skia-graphite-chromes/).
- WebRender: opaque front to back with depth, alpha back to front;
  picture caching per scroll root; primitives interned with relative
  coordinates: [WebRender wiki, "Path to the Screen"](https://github-wiki-see.page/m/servo/webrender/wiki/Path-to-the-Screen),
  [`picture.rs` docs](https://doc.servo.org/src/webrender/picture.rs.html),
  [`rectangle_occlusion.rs`](https://doc.servo.org/src/webrender/rectangle_occlusion.rs.html),
  [Mozilla bug 1628530](https://bugzilla.mozilla.org/show_bug.cgi?id=1628530),
  [Mozilla bug 1558106](https://bugzilla.mozilla.org/show_bug.cgi?id=1558106).
  WebRender's edge antialiasing method was not found in these sources
  (not verified).
- Vello: compute-centric, analytic coverage, no fixed-function MSAA;
  sparse strips as the hybrid successor, experimental:
  [vello sparse_strips README](https://skia.googlesource.com/external/github.com/linebender/vello/+show/6a45f75661129aa3dcbfeeaed5bbdae2451b880e/sparse_strips/README.md),
  [`vello_gpu` docs](https://docs.rs/vello_gpu),
  [Levien's 2025 slides](https://docs.google.com/presentation/d/1f_vKBJMaD68ifBO2j83lBly9Zdk-2bsvj_DIHXxvcuk/htmlpresent).
- Pathfinder: CPU tiling prepass, exact area coverage, occlusion of
  fully covered tiles: [servo/pathfinder](https://github.com/servo/pathfinder).
- Slug: exact glyph coverage from quadratic outlines on the GPU; public
  domain since March 2026: [Lengyel, i3D 2018 slides](https://terathon.com/i3d2018_lengyel.pdf),
  [terathon.com, "A decade of Slug"](https://terathon.com/blog/decade-slug.html),
  [a Rust and WebGPU implementation](https://gabdube.github.io/articles/rust_slug/rust_slug.html).
- MSAA on tile GPUs: 4x at 1 to 2% with resolve attachments and
  `DONT_CARE` store; explicit resolve costs 4x bandwidth; TAA's trade
  (Arm): [Arm Vulkan SDK, "Multisampling"](https://arm-software.github.io/vulkan-sdk/multisampling.html),
  [Arm, "Temporal anti-aliasing"](https://developer.arm.com/community/arm-community-blogs/b/mobile-graphics-and-gaming-blog/posts/temporal-anti-aliasing),
  [Arm, "How low can you go: building low power, low bandwidth Mali GPUs"](https://developer.arm.com/community/arm-community-blogs/b/mobile-graphics-and-gaming-blog/posts/how-low-can-you-go-building-low-power-low-bandwidth-arm-mali-gpus);
  Meta on Quest: [GPU improved algorithms](https://developers.meta.com/horizon/documentation/unreal/gpu-improved-algorithms/).
  Apple GPUs were not covered by these sources (not verified).
