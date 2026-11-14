# Target state: HTML, CSS, SVG and canvas in 3D scenes

Status: vision, 2026-11-14. Nothing here is built or scheduled; it
changes no current work. Read from `origin/main` at 9c132e17. Every
cost is unmeasured unless a file is cited; each slice in section 6
names what it measures.

Tom's questions: "will I be able to lay out a field of 3d objects with
css grid, do svg animations on the side of a sphere, put an html form
around a cone?" and "should we support canvas?"

Short answers at the target state: yes to the grid, on a plane of the
scene (CSS has no third layout axis); yes to animated SVG on a sphere and
to a form around a cone, both drawn exactly by the same primitives as
top-level content (Tom, 2026-11-14), the cone with a stated horizontal
distortion that grows towards the apex (section 3.2). Canvas: no Canvas
2D context; instead a pixel buffer primitive whose writes name their
regions, with sparse tiles and tile-level differencing for touché's
layers (section 4).

Standing rules every part keeps: analysable sources only
(`docs/decisions/2026-11-11-analysable-sources.md`); a small shared
runtime, never per-use copies; idle does nothing; an edit's cost is
flat in scene size; a transform change writes one entry (one uniform
today, `crates/scene3d-gpu/src/slots.rs`; one 64-byte transform-table
entry under `docs/plans/flat-scene.md` 2.2); every failure loud;
3D may become suitable for games and nothing here rules that out.

## 0. Where things stand

- **One-scene D1 (a)** (`docs/plans/one-scene.md` section 5, Tom
  2026-11-12): for property 9, HTML is painted over the scene's
  `gpu-surface` texture. D1 (b), one display list, was taken up by
  `docs/plans/flat-scene.md` (Tom, 2026-11-13, D1 to D10 as
  recommended) as the direction after property 9: one primitive record for
  boxes, text, shapes and meshes; transform and clip tables read per
  instance; a scene as a depth range of the frame's one pass (1.3);
  MSAA 4x on pages with a mesh pipeline (D2); glyph outlines under
  rotation or projection (4.4, regime C); `PaintUses`, a per-crate
  table of the paths a page can need (3.2). Its slice F12 puts a flat
  HTML subtree in a scene. This document starts where F12 ends.
- **CSS on scene nodes**: `docs/plans/css-for-3d.md` C1 to C3 are
  built. Its section 3 refuses the box model, `position` and CSS
  `transform` on scene tags. C4, the 3D hit test, is not built.
- **SVG in 3D**: one-scene D2 (a), a flat `<Decal>`; D2 (b), SVG as a
  texture material, is "3D slice 3b, not planned yet". The shape chain
  (`crates/shapes`, one-scene S1 to S8) is in progress; full SVG is
  `docs/plans/svg-support.md`.
- **Textures in 3D**: none. `3d-engine-parity.md` M2 plans
  `Unlit { texture }` (E9: image pipeline or app texture). An
  after-launch item (lead state, 2026-11-14) scopes a texture
  material, previous-frame feedback and fixed-timestep video capture.
- **Canvas**: no `<canvas>` element. `crates/canvas` is touché's image
  view, not the HTML element. `gpu-surface` gives an application
  renderer the device and an encoder over a box
  (`docs/gpu-surface.md`, "Renderer contract").

## 1. CSS layout placing 3D content

### 1.1 The panel

The target state has one scene component for 2D content in the scene,
called `<Panel>` here:

```html
<Scene scene={scene} camera={eye}>
  <Panel transform={tilt} units_per_px={0.01} size={[1200.0, 800.0]}
         surface={Surface::Plane}>
    <div class="shelf">
      {for item in items.get() key=item.id}
        <div class="cell"><Mesh geometry={item.shape} /></div>
      {/for}
    </div>
  </Panel>
</Scene>
```

- A panel is a scene node and the root of an HTML subtree laid out in
  CSS px by the existing layout, its initial containing block `size`
  px; `units_per_px` maps px to scene units. Every CSS property HTML
  takes applies inside it, grid and flex included; `css-for-3d.md`
  section 3 is unchanged for scene tags.
- Its primitives are flat-scene primitives under the panel's matrix,
  in the scene's depth range (F12's route). A `<Decal>` (one-scene S8)
  is a panel holding an `<svg>` (decision D7).

### 1.2 What a layout box means in 3D

A box laid out in a panel is a rectangle in the panel's local XY
plane, in px, at z = 0. It has no depth. Layout never reads or writes
z, and CSS has no layout along a third axis: Chromium's `preserve-3d`
places flat boxes by `transform` only, after layout
(`docs/design/09-3d.md`, stage 1). A "volume" layout is therefore
several panels, each placed by its `transform` prop, or a `{for}`
computing transforms. Inventing a third-axis grid would be a vapors
grammar with no specification and no browser oracle (decision D2).

### 1.3 3D objects as grid items

A `<Mesh>` inside an element in a panel is placed at that element's
content box. The mesh's scene parent is the panel node; between them
sits a placement entry that layout writes:

```
world = panel_world * scale(units_per_px) * placement * local
placement = translate(content box origin) * fit(geometry AABB, content box)
local = the mesh's `transform` prop
```

- `fit` maps the geometry's axis-aligned bounding box into the content
  box as CSS `object-fit` maps an image: `contain` (uniform scale, depth
  scaled by the same factor), `none` (scene units, centred), with
  `object-position` for alignment. These two properties join the mesh
  tag's property set (`css-for-3d.md` 4.2), read only when the mesh's
  anchor is inside a panel and refused elsewhere naming `<Panel>`.
- The mesh has no natural size and gives layout nothing, as a
  `gpu-surface` has none (`docs/gpu-surface.md`, "The element"). The
  cell's size comes from CSS (grid tracks, flex basis). So a field of
  3D objects is a CSS grid of cells, one mesh per cell.
- One writer per entry: layout writes `placement`, the prop writes
  `local`. A hover rule that grows a cell moves its mesh through
  layout; a spin is the `transform` prop. Neither writes the other.
- Costs: a relayout moving k boxes writes k entries (64 bytes each);
  a transform prop change writes one; idle none. Layout costs what it
  costs on a 2D page of the same boxes. The alternative is D1 (b).

## 2. Drawing content in a scene: exact by default

Tom, 2026-11-14: "rasterising could be a future option, but let's go
with exact rendering as a default." Content in a scene is drawn by the
same primitives and pipelines as top-level content: boxes (analytic
coverage, `quads.wgsl`), glyphs (atlas or outlines by flat-scene's
regimes) and shapes (the shape table). Nothing is first drawn into a
texture and resampled.

### 2.1 Routes by surface

| Surface | Route | How | Cost unmeasured? |
|---|---|---|---|
| Plane | flat-scene F12 | primitives under the node's matrix in the transform table | measured by F12 |
| Cylinder, cone, sphere | M, mapped primitives | boxes, glyph quads and shape meshes subdivided at the shape table's tolerance and mapped in the vertex shader through a surface map `S(u, v)`; coverage stays each primitive's own (2.2) | yes: vertex counts after subdivision (X6) |
| An arbitrary mesh with UVs (glTF) | F, fragment evaluation, an option | the host mesh's fragment shader evaluates vector coverage at its UV from curve bands (Slug's method, `flat-scene.md` D4 (b)); HTML would need its boxes and glyphs in the same band form | yes, entirely (decision D3) |
| Any, as a future non-default option | T, texture | content rendered into a texture sampled by the mesh's material (2.3) | yes |

### 2.2 Route M in detail

- A surface map is a closed enum, `Surface::{Plane, Cylinder { radius,
  arc }, Cone { r0, r1, height, arc }, Sphere { radius }}`, so the
  compiler sees which maps a page can draw (`PaintUses` gains a field)
  and a map is a few lines of shared WGSL, not per-use code.
- Subdivision: a straight edge of chord length c on a surface of
  radius r departs from the surface by about c² / 8r. Box quads, glyph
  quads and shape-mesh edges are split until that is under the shape
  table's tolerance at the largest scale the entry is drawn at
  (`svg-support.md` D1 (a)); for a 16 px feature on a 200 px radius
  that is 0.16 px before splitting. A shape is split at tessellation
  time (counted with `shapes_tessellated`); a box or glyph quad is
  split when its item is built, per surface and size, and kept with
  the item's bytes.
- Coverage after the map: a box evaluates its rounded-rectangle
  distance in the panel's px coordinates, interpolated from the
  vertices, with `fwidth`, as `quads.wgsl` does today; shapes use S2's
  per-triangle edge distances; text under a surface map is projected
  text, flat-scene regime C (glyph outlines through the shapes
  pipeline, F6), with regime B's rule for animated motion (F5).
- Animation by CSS: `fill`, `stroke` and `opacity` transitions write
  paint bytes (svg-support V21); a `transform` on an SVG element or a
  box inside a mapped panel is a transform in the panel's px space,
  applied before the map in the vertex shader, so moving content across
  the sphere writes one entry per frame and re-maps nothing on the CPU.
  `stroke-dashoffset` tessellates its shape per changed frame (V4),
  counted.
- A sphere's map has singular poles: content within a stated angle of
  a pole is refused naming the angle (set from a measurement in X4),
  not drawn pinched.

### 2.3 Route T, a future option

For what route M cannot reach: an arbitrary mesh with no surface map,
and effects that need a texture (previous-frame feedback, capture to
video, the after-launch item). Content is rendered into a texture as a
layer context (`flat-scene.md` 1.1, `svg-support.md` V11) and sampled
by `Unlit { texture }` (`3d-engine-parity.md` M2). Re-rendering is by
dirty region: a change marks the tiles its primitives cover, as the
pixel buffer marks its dirty tiles (4.3), and only those tiles are
drawn again and their mips rebuilt. It is soft when the camera comes
closer than its texel density allows, which is why it is not the
default. Its resolution rule is decision D4.

## 3. HTML in the scene

### 3.1 Flat panels

Flat-scene F12 gives the drawing and the click. The target state adds:

- **Text quality.** Under projection text is regime C: glyph outlines,
  no hinting, coverage through text's `adjust()` (`flat-scene.md`
  4.4). Body sizes are measured in F6 and refused until they pass
  (flat-scene risk 4). The at-rest oracle for a flat panel is Chromium
  drawing the same subtree under the same `matrix3d()`, which
  resamples a layer raster; the comparison states that difference.
- **Depth.** A panel's primitives are depth-tested in the scene's
  range, drawn in paint order after the opaque meshes with depth write
  off, then the panel's box is written into depth by one depth-only
  draw, so a blended mesh behind it is rejected. Panels are blended
  content (antialiased edges) and sort with blended meshes by origin
  depth, with `css-for-3d.md` D4's stated error for intersecting ones.
- **Input.** The scene's ray (C4) finds the nearest of the meshes and
  the panels' planes; on a panel, the hit point goes through the
  panel's inverse matrix to px and then through the document's kept
  hits (`crates/elements/src/painted.rs`, `hit`). Events, capture and
  `:hover` then run unchanged. A captured pointer's moves map onto
  the panel's infinite plane; a ray parallel to it gives an event whose
  offset is absent and says so, never a guessed one.
- **Focus and keyboard.** Elements in a panel are document elements:
  Tab order, focus, `:focus-visible`, text fields and IME work as on a
  page. The platform's IME candidate window takes the caret's
  projected bounding rectangle. vapors does not move the camera to a
  focused element; the app listens for focus on the panel (decision
  D6).
- **Accessibility.** Panel elements are in the tree (scene anchors are
  not). Mirror bounds are relative to the parent's
  (`crates/a11y/src/lib.rs`, `AxMirrorNode::bounds`), so on the web the
  panel's mirror element carries a `matrix3d()` and a camera move
  writes that one element. Natively each node takes a 2D bounding box
  (AccessKit's node transform is 2D affine as known here), recomputed
  per camera settle: O(panel nodes) per settle. Occlusion by meshes is
  not modelled (decision D6).

### 3.2 Wrapped panels: a form around a cone

A panel with `surface={Surface::Cone { .. }}` lays out in px as a flat
panel does, and its primitives are mapped through the surface map
(2.2): boxes and glyph outlines are subdivided and mapped in the vertex
shader, with each primitive's own coverage (2.2).

What each surface does to lengths:

| Surface | Unrolls flat | What layout px become |
|---|---|---|
| Plane | yes | exact |
| Cylinder | yes, to a rectangle | exact: x is arc length, y is height |
| Cone (frustum band) | yes, to an annular sector | y is slant distance, exact; x is mapped to angle, so horizontal lengths scale by s / s_mid at slant distance s from the apex: a band from r0 = 90 to r1 = 110 is within 10% of its laid-out width; a band reaching the apex is not usable |
| Sphere | no (no map from the plane to a sphere preserves lengths) | every map stretches; text near the poles distorts |

So: a form around a cone works, wrapped on a band whose radius ratio
the author chooses, with the horizontal scale stated in the docs and
`wisp-check` printing each wrapped panel's worst ratio. A sphere takes
HTML and SVG drawn the same way, its stretch stated and printed and a
cap around each pole refused (decision D5).

Hit testing on a wrapped panel: ray against the analytic surface (a
quadratic for cylinder, cone and sphere), the inverse map to (u, v),
then px, then the kept hits. On an arbitrary mesh (routes F and T):
ray against triangles, barycentric UVs from the geometry `Meshes`
holds on the CPU (`crates/scene3d/src/meshes.rs`), then px.

### 3.3 Existing practice

As known here from documentation, not checked today; each is checked
before a slice copies a behaviour.

- **three.js `CSS3DRenderer`**: DOM elements placed by CSS 3D
  transforms in a layer over or under the WebGL canvas; meshes cannot
  occlude them; flat only. Same placement idea; no shared depth.
- **Unreal's widget component**: UMG widgets rendered to a texture on a
  plane or a cylinder, input by ray: rasterised, where vapors' default
  draws the primitives on the cylinder.
- **Unity's world-space canvas**: UI drawn as meshes with depth, input
  by a ray caster; flat only. Similar to the flat panel.
- **WICG "HTML-in-Canvas"** (a Chromium proposal): DOM subtrees drawn
  into a canvas or used as a WebGL or WebGPU texture, the app supplying
  the transform for hit testing: rasterised, with app-side input.
- No browser draws HTML on a curved surface, so 3.2 has no browser
  oracle; the reference is an 8x exact-coverage render
  (`flat-scene.md` 4.5).

## 4. Canvas and pixel buffers

### 4.1 What is already covered

- **WebGPU drawing**: `gpu-surface` (device, a vapors-owned encoder,
  a backing texture the box's size, counted work). Differences from a
  web `<canvas>` with `getContext("webgpu")`: no swap chain to
  configure, vapors composites the result, no WebGL context.
- **Charts and diagrams**: SVG with bound geometry (`d={expr}`,
  one-scene S6) and `{for}` over data, in 3D on any route of section 2.
- **Pixels the app computes** (touché's layers, a paint program, an
  emulator's frame): not covered today; 4.3 is the primitive for them.

### 4.2 Why not a Canvas 2D context

The analysable-sources rule closes ways to change styles, classes and
custom properties that the compiler cannot see. A Canvas 2D context
changes pixels inside its own box, as a `gpu-surface` renderer does,
so drawn into a surface it breaches nothing. Drawn into the flat list
it would: `PaintUses` could not name its pipelines, so every page
holding one would build them all. Its drawing operations (paths, text,
composite modes) duplicate SVG with bound geometry, and its
immediate-mode redraw costs the canvas's content per change, not the
change. Tom, 2026-11-14: a pixel buffer primitive is wanted, for a
narrower use than canvas. Decision D8.

### 4.3 The pixel buffer

A `Pixels` store beside `Meshes` issues `PixelBuffer` handles, counted
and released as `GeometryHandle`s are (`docs/scene3d.md`). A buffer
has a format from a closed enum (`Rgba8UnormSrgb`, `Rgba16Float`,
`R8Unorm` for masks), a size in pixels and a tile size (256 by
default, touché's, `~/projects/touche/DESIGN.md` section 3).

**Where writes come from.** Only through the store, each write naming
its region, so the runtime knows every changed pixel's tile:

- `write(region, bytes)`: CPU bytes, refused when the length is not
  the region's area times the format's size;
- `fill(region, colour)` and `copy(from, region, to)`, a copy between
  buffers of one format being a GPU copy with no CPU bytes;
- `render(region, pass)`: a GPU pass the app records (a brush dab, a
  heal) that writes only inside the declared region, which the buffer
  enforces with a scissor and viewport.

There is no drawing API: paths and text are SVG and HTML. The compiler
sees each place a buffer is shown (an element or prop of the closed
handle type) and adds the formats to `PaintUses`; it does not see pixel
values, as it does not see an image's.

**Uploads.** Each write marks its tiles dirty with the written
rectangle; the next frame uploads each dirty tile's rectangle once
through `WorkCounter::write_texture` (`crates/renderer/src/work.rs`),
counted, as the atlas uploads regions (#160). Idle uploads 0 bytes; a
write covering k tiles uploads at most its own bytes. CPU memory holds
the buffer's bytes only for CPU-written buffers; a buffer written only
by GPU passes has no CPU copy. Mipmaps, when a consumer samples
minified, are rebuilt for the dirty tiles' footprint only (region mips
are not built; `crates/renderer/src/mips.rs` rebuilds whole images
today; cost unmeasured).

**Where it is shown.**

| Consumer | How | A write costs besides the upload |
|---|---|---|
| A 2D box | `<pixels buffer={b}>`, a replaced element with the buffer's size as its natural size, drawn by the image pipeline with nearest or linear sampling | no restyle, no layout, no paint bytes: the frame redraws the batch that samples it |
| A `gpu-surface` | the renderer binds the buffer's texture view (one device, `docs/design/09-3d.md`, "Textures are the engine's"); a write invalidates the surfaces that read it | the surface's own render |
| A 3D material | `Unlit { texture }` takes a buffer as an app texture (`3d-engine-parity.md` E9): the buffer is the content, so there is nothing to draw exactly instead | the meshes that sample it redraw; no node slot written |

A buffer past the device's texture edge (16,384 on the dev machine's
adapters, `docs/gpu-limits.md`; 8,192 by default on the web) is
refused when dense; the sparse form (4.4) has no such edge.

### 4.4 Sparse buffers and differencing, for touché's layers

Touché's design already stores pixel layers as 256 px tiles, content
addressed, so a heal touching four tiles stores four tiles and a
duplicated layer costs nothing until painted (`DESIGN.md` section 3,
"Design consequences"); undo appends compensating events that
reference tiles by hash (section 2, "State"). Touché has no pixel
layer yet (`crates/document/src/layer.rs` has `Adjustment` only). The
two vapors features below would give that design a GPU side.

**Sparse.** A sparse buffer allocates a tile only where written; an
unwritten tile reads as the buffer's declared constant (transparent, or
a base buffer's tile, below). WebGPU has no sparse textures, so the GPU
form is an atlas texture of tile slots plus an index from tile
coordinate to slot, read in the shader: one extra texture read per
sample. Bilinear filtering across tile edges needs a one-pixel border
per stored tile (258 x 258 for a 256 tile, 1.6% more memory), and mips
need a border per level; both are unmeasured in cost.

**Differencing.** A buffer may name a base: a tile it has not written
reads the base's tile. That is copy-on-write at tile granularity, which
is touché's content-addressed design expressed in the buffer. Tiles are
shared by handle: two buffers holding the same tile handle hold one
CPU copy and one GPU slot. Per-pixel differencing (storing new minus
base inside a tile) is the alternative; it saves memory only when a
change is sparse within a tile and costs a second sample per read
(decision D11).

Costs, for touché's worked size (60 MP, rgba16float, 8 bytes a pixel,
`DESIGN.md` section 3):

| Case | Dense | Sparse, shared tiles |
|---|---|---|
| One layer | 480 MB | 480 MB (950 tiles of 512 KB for 9,500 x 6,300) plus a 3.8 KB index |
| A duplicate of it | 480 MB | 3.8 KB (the index) |
| A heal touching 4 tiles | a whole layer, or a 480 MB snapshot for undo | 2 MB of new tiles; the old 4 stay referenced by the event stream |
| Upload for that heal | the whole texture, or the dirty rectangle of one texture | at most 2 MB, the dirty rectangles of 4 tiles |
| Undo of the heal | upload the old pixels | 4 index entries (16 bytes) when the old tiles are still resident; their bytes when evicted |
| GPU memory | every layer at full size | the tiles resident, under a slot budget refused loudly when full |

What touché's layers and undo would get: duplicate and derived layers
at the cost of an index; history whose memory grows with the tiles
changed, not with the steps times the image; undo and redo that move
index entries rather than pixels while tiles stay resident; full
resolution past the web's 8,192 texture edge without one texture of
the image's size. What stays touché's: content hashing, the tile store
on disk and in OPFS, and the proxy pyramid; vapors issues tile handles
and holds residency, and touché maps its hashes onto handles (decision
D12). Touché's engine composites layers in its own render graph
(`DESIGN.md` section 3), so the buffer is the storage its passes
sample, and the viewer shows the composite through a `<pixels>` box.

### 4.5 Options for canvas

| | What | Costs |
|---|---|---|
| (a) | No Canvas 2D context; the pixel buffer (4.3), SVG with bound geometry, `gpu-surface` | porting Canvas 2D code means rewriting it against these |
| (b) | (a), plus a Canvas 2D crate later as a `gpu-surface` renderer over a 2D rasteriser, with the web's bitmap semantics | a crate and a dependency linked only by apps that use it (`3d-engine-parity.md` G1); redraw cost proportional to content; an API to keep compatible |
| (c) | A retained "picture" of shapes recorded from Rust, drawn in the flat list | a second way to make shapes beside SVG; run-time-only content in `PaintUses` |

## 5. The architecture this implies

### 5.1 What one-scene D1 (b) needs, and who provides it

| Need | Provided by |
|---|---|
| One display list with boxes, text, shapes and meshes | `flat-scene.md` 1.1, F2, F8 |
| One depth buffer; scenes as depth ranges; 2D content in a scene | flat-scene 1.3, F8, F9 |
| Late antialiasing under projection | flat-scene D2 (MSAA on mesh pages), analytic box coverage with `fwidth` (F4a), S2's edge distances for shapes |
| Text under projection and on surfaces | flat-scene regimes B and C (F5, F6) |
| HTML in a flat plane of the scene | flat-scene F12 |
| Render to texture (route T, future) | layer contexts (flat-scene 1.1; svg-support V11, the one layer mechanism) |
| A texture as a mesh material | `3d-engine-parity.md` M2 (`Unlit { texture }`, E9) |
| Previous-frame feedback | two layer textures alternated per frame; the after-launch item |
| 3D hit test | `css-for-3d.md` C4 |
| CSS on scene nodes | css-for-3d C1 to C7 |

### 5.2 What this document adds

`<Panel>` with `units_per_px`, a containing block and a surface map
(1.1); placement entries written by layout, and `object-fit` and
`object-position` on meshes in panels (1.3); surface maps as a closed
enum in a shared vertex-shader chunk, with boxes, glyph quads and
shapes subdivided against curvature (2.2); route T as a future option
with dirty-region re-render (2.3); the
chained hit test (scene ray, surface inverse, kept hits); panel
accessibility bounds (3.1); `PaintUses` fields for surface maps, texture
routes, wrapped text and pixel formats; the `Pixels` store with
region writes, dirty-tile uploads, sparse tiles and bases (4.3, 4.4).

### 5.3 What it replaces

`<Decal>` as its own component (if D7 (a)), and `css-for-3d.md` D6
(a)'s note that a scene's keyboard interaction is the surface's or the
HUD's. `Item::Surface` stops hosting scenes at flat-scene F8 and stays
for application renderers.

## 6. Route from today

Each slice is one PR under about 400 lines (estimate), built by a
fresh agent, each check proven by breaking what it guards; each starts
only after the slices it names and after the next is re-checked
against what it found. X rows come after flat-scene F12. "Rules" names
the performance rules the slice's tests pin.

| # | Slice | Shows a person | Accepted when | Rules pinned | Depends on |
|---|---|---|---|---|---|
| X1 | Panel input: focus, Tab, typing, IME on a flat panel; focus events for the app | a sign-in form on a tilted panel, typed into | Tab reaches the panel's fields in document order; typed text appears at the projected caret; the IME rectangle is the caret's projected box | a camera move restyles 0 elements and lays out 0 boxes; idle 0 frames | F12, C4 |
| X2 | Panel accessibility: the web mirror's `matrix3d()` per panel; projected boxes natively; D6's rule for panels facing away | a screen reader reads the form on the tilted panel, highlight on its pixels | `ax_oracle` holds the panel's tree against the same HTML on a page; a camera move writes one mirror element (counted) | per camera settle O(panel nodes) natively, 1 write on the web | X1 |
| X3 | Meshes in a panel's layout: placement entries, `object-fit`, `object-position` | a CSS grid of 3D objects that reflows when the panel is resized | a 4 x 3 grid's meshes land at the cells' centres (pixel test against `Projection::project`); a resize moving k cells writes k entries | idle 0; a mesh's `transform` change writes 1 entry; a cell's class toggle relays out the panel as a 2D page would | X1 |
| X4 | Route M for SVG: surface maps (cylinder, sphere), shape subdivision, transforms before the map | an animated SVG exact on a sphere at any zoom | edge coverage against an 8x reference at near and far zoom; a shape moved across the sphere writes 1 entry per frame and tessellates 0 | idle 0; a transition writes paint bytes only | F9, S3b, V5 |
| X5 | Route M for boxes: box quads subdivided and mapped, coverage in panel px | cards and buttons wrapped on a column | box edges against an 8x reference on the cylinder; a class toggle writes paint bytes and re-subdivides 0 | idle 0 | X4, F4a |
| X6 | Wrapped HTML: glyphs by regime C mapped on a cylinder; hit testing through the inverse map | a form wrapped around a column, clickable and typable | clicks on each field reach it; text passes the 8x reference rule; `wisp-check` prints the panel's worst scale | as X1 | X5, F6 |
| X7 | The cone and the sphere for HTML: `Surface::Cone`, the stated scales, the pole cap | the form around a cone | the scale at three slant distances equals s / s_mid; content inside the pole cap refused naming the angle | as X1 | X6 |
| X8 | Route F, only on a product need: curve bands evaluated in the fragment shader for meshes outside the surface set | exact vectors on a loaded glTF mesh | its cost measured against route M on a sphere before it is accepted | as X4 | X4; flat-scene D4 (b) |
| X9 | Route T, future and non-default: layer texture as a material, dirty-tile re-render, mips for dirty tiles | previous-frame feedback; a texture on a glTF mesh | a change re-renders only the tiles it covers (counted); idle 0 renders | idle 0; one change re-renders its tiles only | F8, M2, V11, P1 |
| P1 | Dense pixel buffer: the `Pixels` store, `write`, `fill`, `copy`, dirty tiles, `<pixels>` in a 2D box | a paint demo: a brush writes into a buffer shown on the page | a dab uploads its rectangle's bytes only (counted); a wrong byte length refused naming the region | idle 0 bytes; a write restyles 0, lays out 0, writes 0 paint bytes | none (2D, before F12) |
| P2 | Buffer to `gpu-surface` and to a 3D material; `render(region, pass)` with the enforced scissor | the paint demo's canvas on a rotating cube | a write invalidates exactly the surfaces and meshes that read it (counted); a pass drawing outside its region leaves outside pixels unchanged (pixel test) | as P1; no node slot written | P1, M2 |
| P3 | Sparse buffers: tile atlas, index, borders, slot budget refused when full | a 12,000 px image shown at full resolution on the web page | only written tiles allocated (counted `tiles_resident`); edge sampling against a dense reference within the paint rules; the web page past 8,192 px | as P1 | P1 |
| P4 | Bases and shared tile handles: copy-on-write per tile, residency kept by handle | duplicate a layer, heal four tiles, undo, redo | duplicate allocates 0 tiles; the heal 4; undo with tiles resident uploads 0 texture bytes and writes 4 index entries | as P1 | P3 |

X1 to X3 give the grid question its answer; X4 the sphere; X5 to X7
the cone. P1 to P4 need none of the X rows and none of
flat-scene: they work on today's painter, and touché's pixel layers
(its milestone 2, `DESIGN.md` section 3) are their first user.

## 7. Decisions for Tom

Decided by Tom, 2026-11-14: content in a scene is drawn exactly by
default, with the same primitives and pipelines as top-level content;
rasterising (route T) is a future option ("rasterising could be a
future option, but let's go with exact rendering as a default").
Sections 2, 3 and 6 follow it.

One at a time, each with a recommendation. None is needed before
flat-scene F12 lands.

**D1. How 3D objects take part in CSS layout.** (a) A mesh is placed at
its parent element's content box by `object-fit`; the cell is an
ordinary element; the mesh tag gains two properties, read inside a
panel and refused outside (1.3). (b) The mesh anchor is itself a
replaced box taking `width`, margins and grid-item properties: one
element fewer per item, but the whole box model becomes a set the mesh
tag takes inside a panel and refuses outside, decided at run time
where a component boundary hides the nesting. (c) No layout for 3D
content; `transform` only, as today. Recommend (a).

**D2. Layout along depth.** (a) Planes only; depth from the `transform`
prop and several panels for a volume. (b) A vapors third-axis layout
(a depth track list or a stack axis): no specification, no oracle.
Recommend (a).

**D3. Content on a mesh outside the surface set.** Under Tom's
decision, exact by default, a glTF mesh has no surface map. (a) Panels
and decals on such a mesh are refused naming the surface set until a
product needs them; then F is built and measured (X8). (b) Build F
with the surface maps. (c) Route T for them, rasterised, as the one
exception to the default. Recommend (a).

**D4. Route T's resolution, when it is built.** (a) From the content's
screen footprint, re-rendered at camera settle, dirty tiles only
between settles. (b) A fixed resolution declared in the template:
predictable memory, soft or wasteful by distance. Recommend (a), with a
cap refused loudly past the device's texture limit. Not needed until
X9 is scheduled.

**D5. HTML on a sphere.** Drawing is the same on every surface (route
M), so the question is the stretch. (a) Allowed: the stretch stated and
printed by `wisp-check`, a cap around each pole refused. (b) Refused
naming `Surface::Plane` and `Surface::Cylinder`; SVG still allowed.
Recommend (a): the sphere's stretch is the same kind of stated,
author-chosen distortion as the cone's.

**D6. Panels out of view.** (a) Elements of a panel facing away from
the camera or outside the frustum are marked hidden in the
accessibility tree, recomputed per camera settle at O(panels); occlusion
by meshes not modelled; vapors never moves the camera, and the app
listens for focus on the panel. (b) Panels always exposed, bounds
pointing at whatever is drawn there. Recommend (a).

**D7. One component for 2D content in a scene.** (a) `<Panel>` holds
HTML or SVG; `<Decal>` becomes a panel holding an `<svg>`, migrated by a
script. (b) Keep `<Decal>` for SVG and add `<Panel>` for HTML: two
components for one placement rule. Recommend (a), applied when F12
lands.

**D8. Canvas.** (a) No Canvas 2D context; the pixel buffer, SVG with
bound geometry and `gpu-surface` cover its uses. (b) (a), plus a
Canvas 2D crate on `gpu-surface` when a port needs it, linked only by
apps that use it. (c) A retained picture API in the flat list.
Recommend (a), with (b) left open.

**D9. Where a pixel buffer's writes come from.** (a) CPU region
writes, fills, buffer-to-buffer copies and app GPU passes confined to
a declared region (4.3). (b) CPU writes only: simpler, but a brush or
heal computed on the GPU reads back to the CPU to write again.
Recommend (a).

**D10. Sparse tiles in vapors.** (a) vapors' buffer has a sparse form
(tile atlas and index, 4.4), used by touché and any app past the
texture edge. (b) vapors keeps dense buffers; touché tiles in its own
engine. (b) leaves the web's 8,192 edge to every app and gives the
`<pixels>` box no way to show a tiled image. Recommend (a).

**D11. Differencing granularity.** (a) Tile-level copy-on-write with a
base buffer and shared tile handles. (b) Per-pixel deltas inside tiles,
compressed: smaller for sparse changes within a tile, a second sample
per read and a decode per write. Recommend (a); (b) only if touché
measures edits whose tiles are mostly unchanged.

**D12. Who owns tile identity.** (a) vapors issues counted tile handles
and holds GPU residency; touché maps its content hashes onto handles
and keeps the hashed store on disk. (b) vapors hashes tile content
itself: dedup without touché's help, a hash per written tile in
vapors, and a second answer to "which tiles are equal". Recommend (a).

**D13. Placement.** (a) X rows after flat-scene F12 in section 6's
order, X8 on a product need, X9 when the after-launch feedback or
capture item is scheduled; P1 to P4 when touché's milestone 2 needs
pixel layers. (b) X9 earlier, beside `3d-engine-parity.md` M2.
Recommend (a).

## 8. Risks

- **Small text in perspective.** Regime C has no hinting, and text on
  a panel that moves with the camera changes coverage every frame.
  Whether body sizes pass the paint rules is unmeasured (F6).
- **Subdivision counts.** Mapped boxes and glyphs multiply vertex
  counts by the split factor; unmeasured until X6.
- **Tile borders and mips in the sparse form** add per-sample index
  reads and per-level borders; their GPU cost is unmeasured until P3.
- **Native accessibility under perspective.** A bounding box per node
  does not match a tilted element's outline.
