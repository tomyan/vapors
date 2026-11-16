# `<Decal>`: SVG shapes on a plane in the 3D scene (property 9)

Design note, 2026-11-16, read from main at dcfae109. Nothing here is
built. It replaces `one-scene.md` S7 and S8 and refines its 2.3; the
target state beyond it is `docs/vision/target-state-3d.md` (a decal
becomes a `<Panel>` holding an `<svg>` when flat-scene F12 lands, its
D7). Line counts are estimates.

Property 9 (`perf-finish-line.md`, row 9) is judged on a minimal test
scene: a few 3D objects, an SVG decal on one, HTML on top, CSS reaching
scene nodes, no frame over 16.7 ms across 60 s. `crates/one-scene` has
all of it but the decal (`one-scene.md` section 0).

## 1. The component

```html
<Mesh geometry={cube.clone()} class="one" transform={at(-2.0, 0.0, 0.0)}>
  <Decal transform={front_face()} width={1.0} height={1.0}>
    <svg viewBox="0 0 100 100">
      <circle class="roundel" cx="50" cy="50" r="40" />
      <path class="number" d="M44 30 L56 30 L56 70 L44 70 Z" />
    </svg>
  </Decal>
</Mesh>
```

- A decal is a scene node: a rectangle `width` by `height` scene units
  in its local XY plane at z = 0, centred on its origin, facing +Z,
  placed by `transform` in its parent's space. Its one child is an
  inline `<svg>`, whose view box maps onto the rectangle. A view box
  not in the ratio of `width` to `height` is refused naming both, as
  the page refuses an svg whose size is not in its view box's ratio
  (`svg_checks.rs`, `ratio`: `preserveAspectRatio` is not built).
- Its parent is a `<Mesh>` or a `<Group>`. Under a mesh it moves with
  the mesh and inherits the mesh anchor's `color`, so `currentColor` in
  a path's `fill` is the body's colour (`css-for-3d.md` 1.1). `<Mesh>`
  takes children for this, and only `<Decal>` children; anything else
  under a mesh is refused naming the tag.
- The svg's shapes are what an svg on the shapes route draws today:
  `<path>`, `<rect>`, `<circle>`, each fill then stroke
  (`svg_shapes.rs`), from the document's run-time shape cache (S4a).
  A shape outside the view box is refused naming the path, as on the
  page (`svg_shape_checks.rs`: shapes are not clipped yet). The S5
  compile-time tables are not needed: a decal's entries are
  `ShapeRef::Runtime` today and become `ShapeRef::Table` when S5b lands,
  with nothing on the scene side changed.

## 2. How the shapes are drawn: in the scene pass, exactly

Tom decided on 2026-11-14 that content in a scene is drawn exactly by
default, with the same primitives as top-level content; rendering to a
texture is route T, a future option (`target-state-3d.md` section 2).
So a decal is not rendered to a texture. Its shapes are drawn by the
closed-form and mesh shaders, in `SceneRenderer`'s pass, through a
projective transform:

```
clip = view_projection * decal_world * (shape_affine(local), 0, 1)
```

`shape_affine` is the instance's 2D affine from the shape's units to
decal units (the view-box map folded in on the CPU), so a decal's
instance bytes are the 2D route's formats unchanged:
`ClosedFormShape`'s 80 bytes and the mesh item's 48
(`closed_forms.wgsl`, `meshes.wgsl`). `decal_world` is the decal
node's slot, the same 80-byte slot a mesh has (`slots.rs`). The camera
uniform grows from 64 to 80 bytes to carry the target's size in device
pixels, which the margins below need.

What changes in the shaders is the vertex stage and the mesh fragment
stage. The coverage functions (`rect_distance`, `ellipse_distance`,
the corner wedges) stay one copy: each shader file splits into a
coverage chunk shared by a 2D and a 3D vertex stage, joined at
pipeline creation with `concat!(include_str!(..), ..)`.

- **Closed forms.** The fragment already takes the distance's rate
  across the screen from `dpdx` and `dpdy` of the shape-unit position.
  Under a projection that position is interpolated perspective-correct
  and its derivatives are the local Jacobian, so the fragment stage is
  unchanged. The vertex stage pads the quad by one device pixel per
  edge (`padding`) from the affine's constant determinant. Under a
  projection it computes each corner's own Jacobian of screen position
  against shape units instead. That is analytic from `clip` and the
  matrix's first two columns. Because a plane maps lines to lines, the
  padded edge stays at least the smaller of its two corners' margins
  from the true edge.
- **Meshes.** The vertex stage moves each outline corner half a device
  pixel out across its lines through that corner's Jacobian, as above.
  The fragment stage replaces the item's constant inverse transpose
  with the per-fragment Jacobian from `dpdx` and `dpdy` of the local
  position. A line's device distance is its local distance over the
  local units per pixel along its normal, which is the closed form's
  rule.
- **Corners behind the camera.** A corner with clip w at or below a
  small epsilon gets no padding. The rasteriser clips the triangle, and
  the edges it cut need no ramp.

Curves in mesh entries are flattened at a scale, and a camera changes
the scale (section 6).

## 3. Depth and order against meshes

- **Order in the pass.** Opaque meshes first, as today. Then decals,
  after every opaque mesh and before blended meshes when the blended
  pass lands (`css-for-3d.md` 1.4), as `flat-scene.md` 1.3 orders 2D
  content in a 3D context. Decals sort back to front by the view depth
  of their origin, one key per decal. A decal's shapes keep SVG paint
  order, consecutive shapes of one kind in one draw.
- **Depth state.** The depth test is `LessEqual` against the meshes,
  with no depth write. Shapes in one decal overlay by paint order, and
  a mesh in front of a decal hides it.
- **Coplanar faces.** A decal on a face has the face's depth up to
  rasterisation differences. It is lifted by the pipeline's depth bias
  (`DepthBiasState`, constant plus slope scale; WebGPU has both). The
  values are the smallest with no fighting over the test scene's camera
  range and at a 60 degree view, measured in D3 and cited there. The
  alternatives were an offset along the normal in the author's
  `transform`, which depends on the scale and fails at the far end of
  a zoom, and depth `Equal`, which needs both surfaces to rasterise to
  the same depths and they do not.
- **Blending.** Shapes are blended (antialiased edges), colour with
  `SrcAlpha, OneMinusSrcAlpha` and alpha with `One, OneMinusSrcAlpha`.
  A decal over a mesh then leaves alpha 1, and one over empty
  background leaves its coverage, which the painter's composite of the
  surface reads as it reads any surface pixel.
- **Known limits, stated, not refused:** two decals that overlap on
  screen at crossing depths sort per decal, not per pixel. A decal seen
  from behind draws mirrored, as CSS's default `backface-visibility:
  visible` draws a flipped box; `hidden` is not in this note. A decal
  is planar: it does not wrap onto curved or multi-face geometry (that
  is route M's surface maps, `target-state-3d.md` 2.2), and a decal
  larger than its face draws past the face's edge, in the air beside
  it, since nothing clips it to the face.

## 4. Picking through a decal

`pick` tests meshes only (`pick.rs`), so today a ray passes through a
decal to the mesh under it. A pointer over the roundel then hovers and
clicks the cube, its anchor's `:hover` and its group's apply, and the
cube's `pointer-events` decides. Property 9 needs no more.

Hits on the decal's own paths, so that `:hover` on a `<path>` works as
on a page, are a later slice (D7, not on property 9's path): the ray
meets the decal's plane, the point is tested against each shape in
decal units, and a decal hit at or nearer than the mesh hit wins.
Until D7, such a rule is refused rather than left to never match.
When the decal sink first reports a path, it checks each rule that can
match the path's style key, read as style sharing reads it
(`rules_for_key`, `restyle_share.rs`). A rule with `:hover` or
`:active` in its selector, or one declaring `pointer-events`, is
refused, naming the rule and D7. The check runs once per key.

## 5. CSS on the paths

- `fill`, `stroke` and `stroke-width` on the paths come from the
  cascade with the presentation attributes mapped (S4a PR 1).
  `currentColor` resolves through the anchor chain: path, svg, decal,
  then the mesh anchor.
- `visibility` on the decal anchor, or inherited from the mesh, skips
  the decal's draws. A path's own `visibility` skips that shape.
- **What a change costs.** A paint change rewrites that decal's
  instance bytes and tessellates nothing. A decal's `transform` writes
  its one slot. A camera move writes the camera uniform and re-sorts
  decals, O(decals), and re-flattens only past section 6's step. An
  idle frame writes nothing.
- **How the shapes reach the scene.** The elements crate reports them
  as it reports a mesh anchor's appearance (`Dom::watch_appearance`,
  `scene_sinks.rs`). A decal anchor's sink receives the svg's shapes
  (refs acquired for the anchor, the affine in decal units, colours)
  after an update that changed them, and no report otherwise. The
  `<Decal>` component writes them to its node and invalidates the
  surface.
- **How the renderer reaches the entries.** `SurfaceFrame` gains
  `shapes`, the painter's `ShapeStore` read-only: the entry buffer and
  the lookup from a `ShapeRef` to a closed form's geometry or a mesh's
  triangle range. Both renderers share one upload, and the store
  refuses an unregistered ref as it does for the page.

## 6. Flattening under a moving camera

The run-time cache flattens a path's curves at a device-pixel scale
(`svg_shapes_paint.rs`: "anchoring an svg in 3D must pass the larger
scale here"). A decal's scale is the most device pixels per decal unit
over its four corners on screen, clamped at the near plane. The scene
works it out each frame after the camera and world update, which costs
one matrix per decal. When it exceeds 2x the scale the decal's meshes
were flattened at, the next document update acquires them again at the
new scale, counted in `shapes_tessellated`. Shrinking does not
re-acquire. A dolly in from far re-flattens once per doubling, and for
the frame between, curves are flattened at no worse than twice the
0.1 px tolerance. Closed forms are exact at any scale.

## 7. Slices

Each is one PR under about 400 lines, built test first, each check
proven by breaking what it guards. The next slice is checked against
what the last one found.

| # | Slice | Accepted when | Depends on |
|---|---|---|---|
| D1 | Projective closed forms: `closed_forms.wgsl` split into a coverage chunk and 2D and 3D vertex stages; the 3D pipeline through the counted door, built at `SceneRenderer::mount` | the 2D pixel tests unchanged; under an orthographic camera facing the plane at 1 unit per device pixel, the 3D stage matches the 2D route within 1/255 per channel for rects, circles and rounded and elliptical corners; on a plane at 60 degrees under perspective, edges match an 8x render of the same within the S3b rules; a quad crossing the near plane draws without NaN | none |
| D2 | Projective meshes: the same for `meshes.wgsl` | the 2D mesh tests and `gpu_mesh_insides.rs` unchanged; the orthographic test above for the S3b icon set; the 60 degree test; the insides rule (every pixel half a device pixel inside covered) under perspective | D1 |
| D3 | `NodeKind::Decal { width, height, shapes }` in `vapors-scene3d`; `SurfaceFrame::shapes`; `SceneRenderer` draws decals after opaque meshes, sorted back to front, `LessEqual` with depth bias, blended | through the Rust API (no `.wisp`): a decal on a cube face, no fighting over the camera range at 60 degrees and at a grazing 85 degrees, where slope-scaled bias is largest, bias values measured at that worst case and cited in the code; a decal partly off its face drawn on the face and past its edge with no fighting at the edge; a decal behind another cube hidden; one draw per run of one kind per decal (counted); an idle frame writes 0 bytes; a shape ref not registered refused naming it | D2 |
| D4 | Flattening scale per decal (section 6) | a dolly from 1x to 16x re-acquires 4 times (counted); at each step, curve edges within the tolerance of an 8x render; a still camera re-acquires 0 | D3 |
| D5 | `<Decal>` in `.wisp`: the elements crate's decal sink (shapes in decal units, paint from the cascade), the component, `<Mesh>` children | a class toggle on a path rewrites that decal's instance bytes and tessellates 0 (counted); `currentColor` follows the mesh's `color`; a hidden mesh hides its decal; refused, each naming the cause: a decal outside a mesh or group, a decal with no svg, a shape outside the view box, a path `:hover` rule (section 4) | D3 (D4 may land beside it) |
| D6 | Property 9: a decal on one of `one_scene.wisp`'s cubes (a roundel and a number, recoloured by the night token), then the 60 s judged run three times | no frame over 16.7 ms in each of three runs, with decal draws and bytes per frame reported; `perf-finish-line.md` row 9 marked; hovering the decal hovers its cube | D4, D5 |
| D7 | Hits on decal paths, later and not on property 9's path (section 4) | `:hover` on a path under the pointer; a decal hit at the mesh's depth wins; the build-time refusal lifted | D5 |

Order: D1, D2, D3, then D4 and D5 side by side, then D6. About 1,900
lines across D1 to D6.

## 8. Decisions taken here (implementation, recorded)

1. Exact drawing in the scene pass, not a texture: Tom's 2026-11-14
   default, so it was not open.
2. Coplanar depth by the pipeline's bias, set from a measurement
   (section 3).
3. Picking passes through decals for property 9, with rules that would
   need path hits refused until D7 (section 4).
4. Flattening re-acquired at 2x steps of scale (section 6).
5. Decals attach under `<Mesh>` or `<Group>` (section 1).
6. Seen from behind, a decal draws mirrored; decals sort back to front
   per decal (section 3).
