# Target state: HTML, CSS, SVG and canvas in 3D scenes

Status: vision, 2026-11-14. Nothing here is built or scheduled; it
changes no current work. Read from `origin/main` at 9c132e17. Every
cost is unmeasured unless a file is cited; each slice in section 6
names what it measures.

Tom's questions: "will I be able to lay out a field of 3d objects with
css grid, do svg animations on the side of a sphere, put an html form
around a cone?" and "should we support canvas?"

Short answers at the target state: yes to the grid, on a plane of the
scene (CSS has no third layout axis); yes to animated SVG on a sphere,
crisp through a mapped mesh or softer through a texture; yes to a form
around a cone, with a stated horizontal distortion that grows towards
the apex; on a sphere, HTML is refused (section 3.2). Canvas: no Canvas
2D context in vapors now; section 4 gives the options.

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
  `gpu-surface` texture. D1 (b), one display list, was set aside as a
  new lane. That lane now has a plan: `docs/plans/flat-scene.md` (Tom,
  2026-11-13, D1 to D10 as recommended): one primitive record for
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

## 2. Vector content on curved surfaces

### 2.1 Three routes

| Route | How | Crisp up close | Works on | Animation cost per frame |
|---|---|---|---|---|
| T, texture | The `<svg>` (or panel) drawn into a pooled texture as a layer context (`flat-scene.md` 1.1, `svg-support.md` V11), mipmapped, sampled by the host mesh's material through its UVs | no: texel density is fixed between re-renders | any mesh with UVs, glTF included | the layer re-rendered and its mips rebuilt: proportional to texture area, not to the change (unmeasured) |
| M, mapped mesh | The shape table's triangles, subdivided, mapped in the vertex shader through a surface map `S(u, v)` for a closed set of surfaces (plane, cylinder, cone, sphere), depth-biased against the host | yes: coverage from S2's per-triangle edge distances with `fwidth`, after the map | the closed set only | a paint change writes paint bytes; a UV-space transform writes one entry; a geometry change tessellates that shape (counted) |
| F, fragment evaluation | The host mesh's fragment shader evaluates the vector content's coverage at its UV, from curve bands (Slug's method, `flat-scene.md` D4 (b)) | yes, at any distance | any mesh with UVs | as M for paint and transform; a geometry change rebuilds that shape's band data |

T is one-scene's D2 (b). M reuses the S chain and flat-scene F9 and
adds a map and subdivision; F adds a second coverage method, cost
unmeasured. Decision D3.

### 2.2 Route M in detail

- A surface map is a closed enum, `Surface::{Plane, Cylinder { radius,
  arc }, Cone { r0, r1, height, arc }, Sphere { radius }}`, so the
  compiler sees which maps a page can draw (`PaintUses` gains a field)
  and a map is a few lines of shared WGSL, not per-use code.
- Subdivision: a straight edge of chord length c on a surface of
  radius r departs from the surface by about c² / 8r. Edges are split
  until that is under the shape table's tolerance at the largest scale
  the entry is drawn at (`svg-support.md` D1 (a)); for a 16 px feature
  on a 200 px radius that is 0.16 px before splitting. Splitting is at
  tessellation time and is counted with `shapes_tessellated`.
- Animation by CSS: `fill`, `stroke` and `opacity` transitions write
  paint bytes (svg-support V21); a `transform` on an SVG element is a
  transform in UV space, applied before the map in the vertex shader,
  so moving a shape across the sphere writes one entry per frame and
  re-maps nothing on the CPU. `stroke-dashoffset` tessellates its shape
  per changed frame (V4), counted.
- A sphere's UV map has singular poles: content within a stated angle
  of a pole is refused naming the angle (the number is to be set from
  a measurement in X5), not drawn pinched.

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
shader. Box coverage stays analytic: the fragment shader evaluates the
rounded-rectangle distance in the panel's px coordinates, interpolated
from the vertices, with `fwidth`, as `quads.wgsl` does today.

What each surface does to lengths:

| Surface | Unrolls flat | What layout px become |
|---|---|---|
| Plane | yes | exact |
| Cylinder | yes, to a rectangle | exact: x is arc length, y is height |
| Cone (frustum band) | yes, to an annular sector | y is slant distance, exact; x is mapped to angle, so horizontal lengths scale by s / s_mid at slant distance s from the apex: a band from r0 = 90 to r1 = 110 is within 10% of its laid-out width; a band reaching the apex is not usable |
| Sphere | no (no map from the plane to a sphere preserves lengths) | every map stretches; text near the poles distorts |

So: a form around a cone works, wrapped on a band whose radius ratio
the author chooses, with the horizontal scale stated in the docs and
`wisp-check` printing each wrapped panel's worst ratio. HTML on a
sphere is refused naming `Surface::Plane` and `Surface::Cylinder`;
SVG on a sphere is allowed, since vector art drawn for a sphere is
drawn in its UV space as a globe texture is (decision D5).

Hit testing on a wrapped panel: ray against the analytic surface (a
quadratic for cylinder, cone and sphere), the inverse map to (u, v),
then px, then the kept hits. On route T (a panel as a texture on an
arbitrary mesh): ray against triangles, barycentric UVs from the
geometry `Meshes` holds on the CPU (`crates/scene3d/src/meshes.rs`),
then px.

### 3.3 Existing practice

As known here from documentation, not checked today; each is checked
before a slice copies a behaviour.

- **three.js `CSS3DRenderer`**: DOM elements placed by CSS 3D
  transforms in a layer over or under the WebGL canvas; meshes cannot
  occlude them; flat only. Same placement idea; no shared depth.
- **Unreal's widget component**: UMG widgets rendered to a texture on a
  plane or a cylinder, input by ray. Route T with a cylinder map.
- **Unity's world-space canvas**: UI drawn as meshes with depth, input
  by a ray caster; flat only. Similar to the flat panel.
- **WICG "HTML-in-Canvas"** (a Chromium proposal): DOM subtrees drawn
  into a canvas or used as a WebGL or WebGPU texture, the app supplying
  the transform for hit testing. Route T with app-side input mapping.
- No browser draws HTML on a curved surface, so 3.2 has no browser
  oracle; the reference is an 8x exact-coverage render
  (`flat-scene.md` 4.5).

## 4. Canvas

### 4.1 What is already covered

- **WebGPU drawing**: `gpu-surface` (device, a vapors-owned encoder,
  a backing texture the box's size, counted work). Differences from a
  web `<canvas>` with `getContext("webgpu")`: no swap chain to
  configure, vapors composites the result, no WebGL context.
- **Charts and diagrams**: SVG with bound geometry (`d={expr}`,
  one-scene S6) and `{for}` over data, styled by the cascade, in 3D on
  any route of section 2.
- **Pixel buffers** (touché's image, a paint program, an emulator's
  frame): an image whose pixels the app updates by region, as atlas
  region uploads do (#160), drawn by the image pipeline.

### 4.2 The analysis question

The rule closes ways to change styles, classes and custom properties
that the compiler cannot see. A Canvas 2D context changes pixels inside
its own box and nothing the cascade or layout reads, which is already
true of a `gpu-surface` renderer: the compiler sees the element and its
CSS, not what is drawn in it. What would breach the target design is a
2D context drawing into the flat list itself: `PaintUses` could not
say which pipelines it needs, so every page holding one would build
them all, and its primitives would be invisible to the build-time
proofs of `flat-scene.md` section 3.

### 4.3 Options

| | What | Costs | In 3D |
|---|---|---|---|
| (a) | No `<canvas>`; 4.1's three routes | none; porting existing Canvas 2D code means rewriting it as SVG or a surface renderer | SVG crisp (M, F) or texture; a surface's texture on route T |
| (b) | A Canvas 2D API in its own crate, implemented as a `gpu-surface` renderer over a 2D rasteriser (vello or similar, chosen by measurement), with the web's bitmap semantics (`width` and `height` in pixels, `getImageData`, composite operations) | a crate and a rasteriser dependency, linked only by apps that use it (`3d-engine-parity.md` G1); an immediate-mode redraw per change, proportional to the canvas's content, not flat; the API surface to keep compatible | a texture (route T), soft up close by definition |
| (c) | A retained vapors "picture": shapes recorded from Rust each time inputs change, drawn as flat-list primitives | crisp everywhere, but a second way to make shapes beside SVG with bound geometry, and run-time-only content in `PaintUses` (4.2) | crisp |

Recommendation: (a) now; (b) as a separate crate when a port needs
it; not (c). Decision D8.

## 5. The architecture this implies

### 5.1 What one-scene D1 (b) needs, and who provides it

| Need | Provided by |
|---|---|
| One display list with boxes, text, shapes and meshes | `flat-scene.md` 1.1, F2, F8 |
| One depth buffer; scenes as depth ranges; 2D content in a scene | flat-scene 1.3, F8, F9 |
| Late antialiasing under projection | flat-scene D2 (MSAA on mesh pages), analytic box coverage with `fwidth` (F4a), S2's edge distances for shapes |
| Text under projection | flat-scene regime C (F6) |
| HTML in a flat plane of the scene | flat-scene F12 |
| Render to texture | layer contexts (flat-scene 1.1; svg-support V11, the one layer mechanism) |
| A texture as a mesh material | `3d-engine-parity.md` M2 (`Unlit { texture }`, E9) |
| Previous-frame feedback | two layer textures alternated per frame; the after-launch item |
| 3D hit test | `css-for-3d.md` C4 |
| CSS on scene nodes | css-for-3d C1 to C7 |

### 5.2 What this document adds

`<Panel>` with `units_per_px`, a containing block and a surface map
(1.1); placement entries written by layout, and `object-fit` and
`object-position` on meshes in panels (1.3); surface maps as a closed
enum in a shared vertex-shader chunk, with subdivision against
curvature (2.2, 3.2); route T's texture binding, its resolution from
the screen footprint and re-rendered at camera settle (D4); the
chained hit test (scene ray, surface inverse, kept hits); panel
accessibility bounds (3.1); `PaintUses` fields for surface maps, texture
routes and wrapped text.

### 5.3 What it replaces

`<Decal>` as its own component (if D7 (a)), and `css-for-3d.md` D6
(a)'s note that a scene's keyboard interaction is the surface's or the
HUD's. `Item::Surface` stops hosting scenes at flat-scene F8 and stays
for application renderers.

## 6. Route from today

Each slice is one PR under about 400 lines (estimate), built by a
fresh agent, each check proven by breaking what it guards; each starts
only after the slices it names and after the next is re-checked
against what it found. All come after flat-scene F12. "Rules" names
the performance rules the slice's tests pin.

| # | Slice | Shows a person | Accepted when | Rules pinned | Depends on |
|---|---|---|---|---|---|
| X1 | Panel input: focus, Tab, typing, IME on a flat panel; focus events for the app | a sign-in form on a tilted panel, typed into | Tab reaches the panel's fields in document order; typed text appears at the projected caret; the IME rectangle is the caret's projected box | a camera move restyles 0 elements and lays out 0 boxes; idle 0 frames | F12, C4 |
| X2 | Panel accessibility: the web mirror's `matrix3d()` per panel; projected boxes natively; D6's rule for panels facing away | a screen reader reads the form on the tilted panel, highlight on its pixels | `ax_oracle` holds the panel's tree against the same HTML on a page; a camera move writes one mirror element (counted) | per camera settle O(panel nodes) natively, 1 write on the web | X1 |
| X3 | Meshes in a panel's layout: placement entries, `object-fit`, `object-position` | a CSS grid of 3D objects that reflows when the panel is resized | a 4 x 3 grid's meshes land at the cells' centres (pixel test against `Projection::project`); a resize moving k cells writes k entries | idle 0; a mesh's `transform` change writes 1 entry; a cell's class toggle relays out the panel as a 2D page would | X1 |
| X4 | Route T: a layer's texture as a mesh material (M2), re-rendered on change, mips, resolution from footprint at settle | an SVG diagram on the side of a rotating sphere | the texture re-renders only on a content change or a footprint settle (counted `layers_rendered`); the after-launch feedback example runs on it | idle 0 renders; a rotation renders 0 layers while it moves and at most 1 at settle; a colour change renders 1 | F8, M2, V11 |
| X5 | Route M: surface maps (plane, cylinder, sphere), subdivision, UV-space transforms | an animated SVG crisp on a sphere at any zoom | edge coverage against an 8x reference at near and far zoom; a shape moved across the sphere writes 1 entry per frame and tessellates 0 | idle 0; a transition writes paint bytes only | F9, S3b, V5 |
| X6 | Wrapped HTML: boxes and glyph outlines mapped on a cylinder; hit testing through the inverse map | a form wrapped around a column, clickable and typable | clicks on each field reach it; text passes the 8x reference rule; `wisp-check` prints the panel's worst scale | as X1 | X5, F6 |
| X7 | The cone: `Surface::Cone`, the stated horizontal scale, the sphere refusal for HTML | the form around a cone | the scale at three slant distances equals s / s_mid; HTML on a sphere refused naming the alternatives | as X1 | X6 |
| X8 | Route F, only on a measurement: curve bands evaluated in the fragment shader | crisp vectors on a loaded glTF mesh | only if X4's softness or X5's limits are measured as a product problem | as X5 | X5; flat-scene D4 (b) |

X1 to X3 give the grid question its answer; X4 and X5 the sphere; X6
and X7 the cone. Canvas is outside this order (section 4).

## 7. Decisions for Tom

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

**D3. Which route draws vectors on curved surfaces first.** (a) T
first (it serves any UV mesh and the after-launch texture material),
then M for the closed set of surfaces where crispness is wanted. (b) M
only: always crisp, no glTF meshes. (c) F for everything: crisp on any
mesh, a second coverage method, cost unmeasured. Recommend (a).

**D4. Texture resolution on route T.** (a) From the panel's screen
footprint, re-rendered when the camera settles (Chromium's re-raster
at the end of an animation). (b) A fixed resolution declared in the
template: predictable memory, soft or wasteful depending on distance.
Recommend (a), with a fixed cap refused loudly past the device's
texture limit.

**D5. Which surfaces take HTML.** (a) Plane, cylinder and cone, the
cone's horizontal scale stated and printed; HTML on a sphere refused;
SVG on all four. (b) All four, distortion stated. (c) Plane only.
Recommend (a).

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

**D8. Canvas.** (a) No `<canvas>`; `gpu-surface`, SVG with bound
geometry and region-updated images cover its uses. (b) A Canvas 2D
crate on `gpu-surface` with the web's bitmap semantics, linked only by
apps that use it. (c) A retained picture API in the flat list.
Recommend (a) now and (b) when a port needs it.

**D9. Placement.** (a) After flat-scene F12, in section 6's order, X8
only on a measurement. (b) X4 (route T) earlier, beside
`3d-engine-parity.md` M2, since the after-launch texture material and
feedback item need the same binding. Recommend (b) for X4 only if the
after-launch item is scheduled before F12; otherwise (a).

## 8. Risks

- **Small text in perspective.** Regime C has no hinting, and text on
  a panel that moves with the camera changes coverage every frame.
  Whether body sizes pass the paint rules is unmeasured (F6).
- **Subdivision counts.** Mapped boxes and glyphs multiply vertex
  counts by the split factor; unmeasured until X6.
- **Native accessibility under perspective.** A bounding box per node
  does not match a tilted element's outline.
