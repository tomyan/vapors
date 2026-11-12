# 3D engine parity: what vapors needs, and how each part costs nothing until used

Status: design, 2026-11-12. For after the performance finish line
(`docs/plans/perf-finish-line.md`): no row here starts until every
property there holds, and nothing here changes current work. Read from
the code at 9b405f9, open PRs #250 to #252, and the plans
`docs/plans/one-scene.md`, `docs/plans/3d-2.md`, `docs/plans/3d-1c.md`
and `docs/plans/particles.md` (branch `docs/particles-plan`, 1733f65).
No build was run for this document. Every number is cited or marked
"not measured", with the method that would measure it.

Sections:

0. The rules this design is bound by
1. What vapors' 3D does today, and after the planned PRs
2. The feature table against Bevy and Godot
3. Each feature: what it takes, what it costs unused, how it stays
   pay-only-if-used
4. What is left out, and why
5. Ranked order and slices
6. Decisions for Tom
7. Risks

## 0. The rules this design is bound by

- **Analysable sources only** (`docs/decisions/2026-11-11-analysable-sources.md`):
  no API whose effect the compiler's build-time analysis cannot see.
  Classes are `class:name={bool}` or an enum, never a string. For 3D
  this means: a node's kind, its material kind, which attributes its
  geometry carries, whether a scene has lights, shadows, blended
  materials or post passes, are all known from the template or from a
  typed prop whose type is closed (an enum, a handle from a store), so
  the renderer can be told at mount what a scene can ever draw.
- **Generated code calls a small shared runtime** (Tom, 2026-11-10,
  the lead's record; `docs/plans/one-scene.md` section 1.4 follows it
  for shapes): no per-use monomorphised copies. For 3D: one pipeline
  per (material kind, vertex layout) in the renderer, shaders composed
  from shared WGSL chunks at pipeline creation, and components that
  write into the scene store; a `.wisp` scene emits calls, not
  renderer code.
- **Runtime performance first** (`docs/plans/perf-finish-line.md`):
  idle draws nothing and does no work (property 6; a surface renders
  only on an invalidation or an animation request,
  `docs/gpu-surface.md`); an edit's cost is flat in scene size
  (property 1's shape: a transform change writes one uniform,
  `crates/scene3d-gpu/src/slots.rs`, #252's 80-byte pin). Every
  feature below says what it does per frame when the scene does not
  change, and what one change costs.
- **Every failure is loud**: refused, naming the file, line, node or
  field and the fix, as `Transform::new` and `Meshes::geometry` refuse
  today (`crates/scene3d/src/transform.rs:33`,
  `crates/scene3d/src/meshes.rs:84`). Nothing is clamped, skipped or
  approximated. A limit is a fixed number raised with a measurement
  (`docs/plans/particles.md`, decision P3).
- **Files about 200 lines, functions 10 to 15, no `unsafe`**
  (`CLAUDE.md`). No `unsafe` rules out crates that need vapors to
  write it (C bindings such as meshoptimizer or Embree); a dependency's
  own `unsafe` is its business and passes `cargo xtask deny` like
  wgpu's.
- **Tom's direction** (`docs/plans/particles.md`, "Decided by Tom"):
  3D in vapors should eventually be suitable for games; nothing here
  may rule that out. This document ranks by what vapors' own apps
  (touché, the racer) need first and keeps the game path open at each
  step, rather than building for a game that does not exist.

## 1. What vapors' 3D does today, and after the planned PRs

### 1.1 Today (9b405f9)

- **Scene store** (`crates/scene3d`, no GPU): `Scene` with generational
  `NodeId`s, `NodeKind::{Group, Mesh { geometry, colour }, Camera {
  handle, projection }}` (`src/node.rs:8`), a `Transform` of
  translation, rotation and scale (`src/transform.rs`), world matrices
  recomputed for changed nodes and their descendants only
  (`src/world.rs`, `Scene::update` returns them), one active camera
  named by handle (`src/camera.rs`), perspective and orthographic
  projections (`src/projection.rs`). `Meshes` validates positions and
  `u32` indices and issues `GeometryId`s (`src/meshes.rs`); a geometry
  has positions only, no normals, UVs, tangents, colours, joints or
  weights. Maths is glam 0.34.3, pinned (`Cargo.toml`).
- **Renderer** (`crates/scene3d-gpu`): a `SurfaceRenderer` on a
  `gpu-surface` (`src/renderer.rs`). One pipeline (`src/pipeline.rs`),
  one shader (`src/shader.wgsl`, 24 lines: position through world and
  view-projection, flat colour), `Depth32Float` depth with `Less`,
  single-sampled, no blending, opaque only. One draw per mesh node
  (`src/draw.rs:59`), one 80-byte uniform slot per mesh node written
  only when its world version changes (`src/slots.rs`). Geometry
  uploaded once per renderer (`src/geometry.rs`). Every write and draw
  goes through `SurfaceFrame::work` into `FrameCounts::surfaces`
  (`crates/app/src/surface.rs:59`). The depth attachment is discarded
  at the end of the pass (`src/draw.rs:92`). No culling, no sorting,
  no lights, no textures, no instancing, no compute, no picking.
- **The racer** (`crates/racer`) still draws with its own `Track`
  renderer and shader (`src/gpu.rs`, `src/track.wgsl`): a procedural
  ground per pixel and one vertex buffer of coloured solids under a
  hand-written tilted orthographic projection. It is the one 3D app.
- **Composition**: the surface's backing is `Rgba8Unorm`, straight
  alpha, composited by the painter in paint order with HTML above it
  (`crates/app/src/surface.rs:10`, `docs/gpu-surface.md`). Input
  reaches a surface in surface-local coordinates with UI above it hit
  first (`docs/plans/3d-1c.md`, PR 5); no `pointerenter`/`leave`
  (same, "Open items from PR 5").
- **Idle**: a surface renders only on `invalidate` or while an
  animation request holds; one request flag per surface
  (`docs/gpu-surface.md`, "Animation"; `particles.md` decision P6 says
  why one flag is not enough).
- **Web**: the racer page's wasm is 7.7 MB uncompressed in the serve
  copy of 2026-11-09 (`~/srv/vapors-site.racer-3d-20261109/racer/`,
  `vapors-web-demo-*_bg.wasm`, 7,709,795 bytes; trunk release build,
  brotli size not recorded there). The precedent for a size-costed
  feature is `complex-script-breaks`: about 2 MB of brotli, measured on
  touché's page with and without it (`crates/text/Cargo.toml:20-24`,
  `docs/design/text-stack-size.md`), and `grid`
  (`crates/elements/Cargo.toml:14-19`). Section 3 uses the same method.

### 1.2 After 3D slice 2 PRs 4, 4b and 5 (`docs/plans/3d-2.md`)

- **PR 4** (#250, #251, #252, open): `vapors-scene3d-components` with
  `<Scene scene camera surface>`, `<Group transform>`, `<Mesh geometry
  colour transform>`, `<Camera handle projection transform>` as Rust
  components with typed props, each with an inert `display: none`
  anchor element; a prop change writes that node alone and invalidates
  the surface (#252: 80 bytes per transform or colour change, pinned).
  Found in #251: a generated `.wisp` component cannot yet be nested in
  another component's children; the compiler change is owed before any
  user-written `<Car>` goes inside `<Group>`.
- **PR 4b**: one cascade into the scene (decision 9): the anchor's
  computed `color`, `opacity` and `visibility` feed its node; `:hover`
  through a 3D hit test; tokens reach the scene.
- **PR 5**: keyed nodes under `{for}` keep identity; unmount removes a
  node and releases geometry no other node uses; unmounting the surface
  drops every GPU resource; `docs/scene3d.md`.

### 1.3 After the one-scene plan (`docs/plans/one-scene.md`, property 9)

- A per-vertex shade factor multiplied by the node's CSS colour (R1,
  D4(d)); no normals, no light.
- An application material: a WGSL fragment given by the app, reading
  world position, on one mesh node (R2, D5(b)). This is the first
  second shading path in `SceneRenderer`, and the extension point the
  3D brief asks for (`docs/briefs/GPU_SURFACE_3D_BRIEF.md`, "Deferred
  work").
- Shapes drawn in the 3D pass from the shape table with a depth bias
  (S7), `<Decal>` with inline SVG (S8), the racer's world in `.wisp`
  with about 11 scene draws (R3a1), the 60 s frame-time run with every
  frame judged (P0, P1).

### 1.4 After the particles plan (`docs/plans/particles.md`)

- The first compute pass, proven on llvmpipe, ANV and SwiftShader (C1,
  C2); `GpuWork::dispatches`; compute timestamps in `GpuTimer` (C3).
- `SurfaceFrame::now` (S1); instanced additive billboards drawn in the
  scene pass after the meshes (S3); animation requests as guards so
  several owners can hold one surface's animation (decision P6).

So, before this document's first row, the renderer has: unlit and
shaded flat colour, one app-given fragment shader, decals from the
shape table, instanced billboards from a storage buffer, compute, a
frame time, and CSS colour, opacity and visibility on nodes. It still
has no normals, textures, lights, culling, sorting, picking beyond
hover, instanced meshes, animation data, or asset loading.

## 2. The feature table

What Bevy and Godot ship as basics, against vapors. Bevy is described
as of 0.16 (`bevy_pbr`, `bevy_gltf`, `bevy_picking`, `bevy_animation`,
`bevy_audio`); Godot as of 4.4. Both from their documentation as I know
it; a row's detail is to be checked against the current docs before
the slice that copies it starts. "Planned" means a row in 1.2 to 1.4.

| Feature | Bevy 0.16 | Godot 4.4 | vapors today | vapors planned | Gap |
|---|---|---|---|---|---|
| Lights | Directional, point, spot; ambient; `StandardMaterial` lit by all | DirectionalLight3D, OmniLight3D, SpotLight3D; environment ambient | none: unlit flat colour | shade factor times colour (one-scene R1): no light direction | normals, a lit material, light nodes |
| Shadows | Cascaded shadow maps (directional), cube maps (point), spot maps; PCF and other filters; contact shadows | Same three kinds, PCF/PCSS, cascades; SDFGI/VoxelGI for indirect | none | none | one directional shadow map (design 09 caps it there) |
| PBR materials | Metallic-roughness per glTF, normal/occlusion/emissive maps, clearcoat, transmission, anisotropy; IBL from an environment map | StandardMaterial3D / ORMMaterial3D, the same maps, subsurface, refraction; sky and reflection probes | none: colour only | app-given fragment (one-scene R2) | a material model: unlit textured, lit, PBR; textures as inputs |
| glTF loading | `bevy_gltf` (the `gltf` crate): scenes, meshes, materials, skins, morphs, animations; async through `AssetServer` | Import at editor time and `GLTFDocument` at run time; full 2.0 with many extensions | none; `Meshes::geometry(positions, indices)` from app code | none | a loader into the stores; geometry attributes beyond position |
| Skinned animation | `SkinnedMesh` with joint matrices in a storage or uniform buffer; `AnimationPlayer`/`AnimationGraph` blends clips | Skeleton3D, AnimationPlayer, AnimationTree (state machines, blend spaces) | none | none | joints and weights attributes, a skin buffer, clip sampling |
| Morph targets | Per-target position/normal/tangent deltas in a texture, weights per mesh | Blend shapes on ArrayMesh, weights per instance | none | none | target deltas in a storage buffer, weights per node |
| Instancing | Automatic: entities with the same mesh and material are batched into one instanced draw (since 0.12); GPU-driven in 0.16 | MultiMeshInstance3D (explicit); automatic instancing in the Forward+ renderer for identical meshes | one draw per mesh node | billboards per emitter (particles S3) | `<Instances>` with keyed `<Instance>`s (brief slice 4) |
| Frustum culling | Automatic from each mesh's `Aabb`, per view, every frame | Automatic AABB culling; occlusion culling (Embree) and portals | none: every mesh node drawn | none | per-geometry AABB, a kept visible set |
| LOD | `VisibilityRange` per entity with crossfade dithering; experimental meshlets | Automatic mesh LOD generated at import (meshoptimizer); visibility ranges with fade | none | none | visibility ranges; no automatic simplification |
| Post-processing | Tonemapping (several), bloom, SSAO, TAA/FXAA/SMAA, depth of field, motion blur, chromatic aberration | Glow, SSAO, SSR, SSIL, DOF, volumetric fog, tonemap, adjustments | none; the surface composites `Rgba8Unorm` straight alpha | none | an HDR intermediate and a pass list, when any is used |
| Transparency | `AlphaMode::{Opaque, Mask, Blend, Premultiplied, Add, Multiply}`; blended entities sorted back to front per view; order-independent transparency (0.15) | Per-object depth sort of transparent materials; no OIT | none: opaque, no blend state | additive billboards, unsorted (particles P5) | alpha modes, a sorted blend list |
| Picking | `bevy_picking` with a mesh ray-cast backend; pointer events on entities with bubbling | Physics ray casts against collision shapes; `_input_event` on CollisionObject3D | none; surface-local pointer events reach the app | `:hover` on a mesh through a 3D hit test (PR 4b) | ray against AABBs then triangles; ordinary pointer events on the anchor |
| Physics hooks | None built in; avian or bevy_rapier as plugins, writing `Transform`s in a fixed-step schedule | Built in (Godot Physics; Jolt in 4.4); `_physics_process` at a fixed tick | application Rust (the racer's model); one transform write per node | ordered tasks (brief slice 5); `SurfaceFrame::now` (particles S1) | batched transform writes; a fixed-step tick in the scheduler |
| Audio hooks | `bevy_audio` (rodio): play, spatial audio from listener and emitter transforms | A bus system with effects; AudioStreamPlayer3D with attenuation | none; no mention in `docs/` (searched) | none | a platform audio service; listener and emitter positions from the scene |
| Asset streaming | `AssetServer` loads asynchronously; no partial texture or mesh streaming | Background `ResourceLoader`; no texture streaming | geometry from app code; images through the painter's image pipeline (`crates/renderer/src/image.rs`, mips) | none | fetch through the platform, validation and upload spread over frames |
| Compute | Yes (`bevy_render`) | Yes (RenderingDevice) | none | first pass (particles C1) | reusable by skinning and sorting |
| Text and vectors in 3D | Text via `bevy_text` meshes or UI; no SVG | Label3D, TextMesh; no SVG | none | decals from the shape table (one-scene S7, S8) | text on a decal (SVG brief slice 7) |

Where vapors already differs on purpose: scene nodes are typed
components in `.wisp`, not entities with components (design 09, "Taken
from where"); CSS sets appearance (colour, opacity, visibility, state
rules) through one cascade (3d-2 decision 9), which neither engine
has; and every frame's writes and draws are counted and pinned in
tests (`FrameCounts::surfaces`), which neither engine does as a
contract.

## 3. Each feature: what it takes, what it costs unused, how it stays pay-only-if-used

### 3.0 The mechanisms, named once

Each feature below names the mechanisms that keep it free when unused.

- **G1, a separate crate.** Code with its own dependencies (a file
  format parser, an audio backend, animation sampling) goes in a crate
  an app depends on only if it uses it, as `vapors-scene3d-gpu` is
  separate from `vapors-scene3d` (3d-2 decision 5) and `vapors-particles`
  is planned separate (particles P7). Unused cost: 0 bytes, 0 per frame.
- **G2, a cargo feature in the renderer.** A pipeline variant that
  needs no new dependency (shadows, blending, HDR) is behind a feature
  of `vapors-scene3d-gpu`, default off, as `grid` is in
  `vapors-elements`. With the feature off, a scene that needs it is
  refused at mount naming the feature and the node, as `display: grid`
  is refused without `grid` (`crates/elements/Cargo.toml:14-19`).
  Unused cost: 0 bytes when off; when on but unused, the code is in the
  binary and 3.0's per-frame rule applies.
- **G3, what the scene can draw is known at mount.** Node kinds and
  material kinds are closed enums, and the compiler sees every
  `<Mesh material={..}>`, `<Light>`, `<Instances>` in a crate's
  templates. A `.wisp` scene therefore mounts with a `SceneUses`
  value (which material kinds, whether any light, shadow, blend, post
  pass, skin or morph can appear), emitted per crate like the planned
  `SHAPES` table (`one-scene.md`, 1.3), and the renderer makes only the
  pipelines those need. A node that appears at run time outside the
  declared set is refused naming the component and the template file.
  This is the analysable-sources rule applied to 3D: nothing a scene
  draws is a surprise to the renderer. Pipelines are then also made at
  mount, not on the first frame that needs them (one-scene D7 risk).
- **G4, one shared pipeline per kind, shaders from chunks.** A
  material kind is one WGSL fragment entry composed at pipeline
  creation from shared chunks (lighting, fog, alpha test) with
  `override` constants for the small choices (light count, whether
  shadows sample). There is no per-material shader string and no
  per-use generated Rust. The 2D painter already keys pipelines by
  kind and keeps them (`docs/plans/iu4.md`, "Kept batches").
- **G5, kept results.** A per-frame answer that depends only on the
  camera and the nodes (visible set, sorted blend list, LOD choice) is
  kept and recomputed only when the camera's version or a tested
  node's world version changes, as paint order and hits are kept in
  the elements crate (`crates/elements/src/kept_hits.rs`). Idle: 0
  work. One node moved: that node retested, O(1). Camera moved: every
  node retested, O(nodes), until a bounding hierarchy (E5) bounds it.
- **G6, fixed budgets refused loudly.** Counts and bytes have a fixed
  limit named in the refusal, raised with a measurement (particles P3).

**How "bloat cost" is measured** for each row, when it is built:

- *Binary size*: `cargo xtask web` on `vapors-web-demo` with and
  without the row's crate or feature, brotli bytes of the wasm as the
  command prints, the method of `complex-script-breaks`
  (`crates/text/Cargo.toml:20-24`). The number goes in the row's PR
  and in `docs/web.md`. Today's reference point: 7.7 MB uncompressed
  for the racer page (1.1). No row below has been measured; each says
  "not measured" and what dominates.
- *Per-frame cost when unused*: a gpu-tier test pins
  `FrameCounts::surfaces` for an idle frame and for one transform
  change on a scene that does not use the feature, before and after
  the row: draws, buffer bytes, dispatches and uniform writes equal.
  `vapors-perf`'s instruction count for the racing workload is the
  CPU-side judge (`perf-finish-line.md`, properties 1 and 6).
- *Per-frame cost when used*: `GpuTimer` time and `FrameCounts` on
  the row's own workload, recorded in `docs/perf/`.

### 3.1 Materials and textures (the prerequisite for most rows)

**What it takes.** A `Materials` store beside `Meshes` issuing
`MaterialId`s, in `vapors-scene3d`; `NodeKind::Mesh { geometry,
material }` replacing `colour` (CSS colour from PR 4b multiplies the
material's base colour; the shade factor from one-scene R1 stays a
vertex attribute). `MaterialKind` is a closed enum: `Unlit { base,
texture: Option<TextureId> }` first; `Lit`, `Pbr` and `App` (one-scene
R2's fragment) join it in their rows. `Geometry` gains optional
attributes, each validated when given and named in the vertex layout:
`uv0`, `normal`, `tangent`, `colour`, later `joints` and `weights`;
a material that reads an attribute the geometry lacks is refused at the
frame naming node, material and attribute (today's `GeometryMissing`
shape, `crates/scene3d-gpu/src/error.rs`). Textures come from the
engine's image pipeline (`crates/renderer/src/image.rs`, with mips)
or are wgpu textures the app owns (design 09, "Textures are the
engine's"); a `TextureId` is issued by a `Textures` store the renderer
uploads once, like geometry. About 350 lines across scene3d,
scene3d-gpu and the components, in two PRs.

**Cost unused.** One more pipeline (unlit textured) only when a
scene's `SceneUses` names a textured material (G3). Binary: a sampler
and bind group layout, under 10 KB of Rust (not measured; no new
dependency). Per frame: 0 when no material changes; a material change
writes that node's slot, 80 bytes plus the material's bytes, flat.

**Why first.** Every lit, PBR, glTF and LOD row needs a material model
and geometry attributes, and touché's layer view (design 09, T3) needs
an unlit textured plane whose texture the engine owns.

### 3.2 Instancing

**What it takes.** `NodeKind::Instances { geometry, material }` with a
kept per-instance buffer (world matrix and colour, 80 bytes per
instance) built on `Retained` from IU4 #247, so one instance's change
writes its 80 bytes alone; `<Instances>` and keyed `<Instance
transform>` components where an `<Instance>` is a row, not a node
(the brief's slice 4: insert, remove, reorder and one-instance change
without touching the others). The racer's 541 trees are the workload
(`one-scene.md`, 2.2). Decision E1 asks whether plain meshes become
instances of one in the same pipeline, which would retire the
dynamic-offset uniform slots (`slots.rs`) for one vertex-stepped
buffer; both routes keep one draw per (geometry, material) group.

**Cost unused.** Under E1(a) no new pipeline; under E1(b) one. Per
frame: 0 when nothing moves; an instance change is one 80-byte write.
Binary: not measured; no dependency.

### 3.3 Lights: a lit material and one directional light

**What it takes.** `normal` attribute (3.1); `NodeKind::Light {
Directional { colour, intensity } }`, later `Point` and `Spot` as enum
variants with a fixed limit (G6: 8 lights per scene first, refused
past it; wgpu's uniform size is not the limit, the shader's loop is);
a `LightsUniform` written when a light's world version or value
changes; `MaterialKind::Lit { base, texture }` with Lambert diffuse and
Blinn-Phong specular, ambient from the scene root. The brief's MVP
asks for exactly this ("basic-lit materials, with one directional
light"). Colours stay sRGB-encoded in the backing (`colour.rs`), so
the lit path converts to linear, shades, and encodes back in the
fragment; HDR (3.9) is not needed for one light at intensity 1.
About 300 lines in two PRs (store and uniform; material and pixel
tests).

**Cost unused.** The lit pipeline exists only when `SceneUses` names a
lit material (G3). A scene with no light nodes and a lit material is
refused at mount naming the fix (an ambient-only scene is written as
unlit). Per frame: 0 with no change; a light change is one uniform
write for the scene, not per node.

### 3.4 Shadows: one directional shadow map

**What it takes.** A depth-only pipeline and a shadow pass before the
scene pass into a `Depth32Float` texture (2048 square first, a fixed
size refused past it until measured, G6), the light's orthographic
view-projection fitted to the camera frustum, a comparison sampler
and a `shadow` chunk in the lit and PBR fragments (G4). Cascades,
point-light cube maps and spot maps are not in this document's
slices: design 09 caps shadows at "one directional shadow map" and
nothing in vapors' apps asks for more. A `<Light shadow={true}>`
toggle is a bool, so the compiler sees it (G3). About 400 lines in
two PRs (the pass; the sampling and a pixel test of a cube's shadow
on a plane).

**Cost unused.** Behind the `shadows` feature of `vapors-scene3d-gpu`
(G2) and made only when a light declares `shadow` (G3). Per frame
when used: the shadow pass draws every caster again, a second draw
per mesh node, and must re-run when any caster or the light moves, or
the camera moves (the fit follows it). With G5 the pass is skipped
on a frame where none of those changed, and the map is kept. On
tiled mobile GPUs the extra pass costs a resolve; the Pixel 9 Pro
number is to be measured in the row.

### 3.5 Frustum culling

**What it takes.** An AABB per `Geometry` computed in
`Meshes::geometry` (24 bytes, always, O(vertices) once at validation);
a world AABB per mesh and instances node recomputed with its world
matrix (the `update` that recomputes world matrices recomputes the
box, same list); a `visible: Vec<NodeId>` kept per (scene, camera)
and recomputed per G5. `FrameCounts` gains `nodes_culled` so a test
pins that an off-screen node is not drawn and an on-screen one is.
Instances are culled per group by the union box first; per-instance
culling is a compute pass (indirect draw) later, after a measurement
shows a group whose union box is on screen while most instances are
off it. About 300 lines.

**Cost unused.** Culling is on for every scene; "unused" is a scene
that fits the view, where it costs one box test per node per camera
change and nothing idle (G5). Binary: no dependency. The O(nodes)
re-test on a camera move is the cost to measure: at 1,000 nodes it is
about 1,000 box-frustum tests, 6 plane dot products each, well under
a millisecond on this machine (not measured; `vapors-perf` instruction
counts are the judge). E5 decides when a hierarchy replaces the scan.

### 3.6 Picking

**What it takes.** PR 4b's `:hover` hit test generalised: a ray from
the camera through the pointer (`Projection::project`'s inverse),
tested against the kept visible set's world AABBs (3.5), then against
the triangles of the candidates in front-to-back order, in the hit
test pass the elements crate already runs for the surface's element.
The hit node's anchor is the target of the ordinary pointer events
(`pointerdown`, `pointerup`, `click`) and of `:hover`, bubbling up the
anchors as elements bubble. A scene whose anchors have no listener and
no state rule is skipped (design 09, "Picking"); the compiler knows
which anchors have either (G3). The `pointerenter`/`leave` gap
(`3d-1c.md`, "Open items from PR 5") is owed before an orbit control
can know the pointer left. About 350 lines in two PRs (ray and AABB
stage with a test that clicks known positions; triangle stage).

**Cost unused.** 0 for a scene with no listeners or state rules. When
used: per pointer move, O(visible nodes) box tests then triangles of
the candidates; a kept hit (the same node under a pointer that moved
within its box) is not re-tested. Binary: no dependency.

### 3.7 Transparency: alpha modes and sorting

**What it takes.** `AlphaMode::{Opaque, Mask { cutoff }, Blend}` on a
material (the glTF trio; Bevy's `Add`/`Multiply` are not needed before
a use). `Mask` is an `alpha < cutoff` discard in the fragment, no
sorting. `Blend` nodes are drawn after the opaque ones with depth
write off, sorted back to front by view-space depth of the node's box
centre, the list kept per G5. Particles' additive sprites (particles
S3) and the decal depth bias (one-scene S7) are drawn in the same
order rule. Order-independent transparency is left out (section 4).
About 250 lines.

**Cost unused.** A blend pipeline only when `SceneUses` names a
`Blend` material (G3); the sort runs only over blend nodes, kept
between frames. Binary: no dependency.

### 3.8 glTF loading

**What it takes.** A crate `vapors-gltf` (G1) over the `gltf` crate
(MIT/Apache-2.0, pure Rust, `cargo xtask deny` to confirm; pinned
exactly as glam is) that reads a `.glb` or `.gltf` with its buffers
and images into the stores: `Meshes` with attributes (3.1),
`Materials` (unlit, lit or PBR as the row allows), `Textures`, a node
tree of `Transform`s, and later skins, morph targets and clips (3.9).
Two routes, decision E4: at build time, the compiler packs a bundled
file into a per-crate static table beside `SHAPES` so a bad file fails
the build naming it and the geometry is a known constant; at run time,
the same loader behind the platform's fetch and `await` for user
content, refusing an unsupported extension, a missing buffer or an
attribute out of range naming the file and the node. Nothing is
approximated: a `KHR_*` extension the row does not implement is a
refusal, not a silent fallback to the base material. About 600 lines
across three PRs (geometry and nodes; materials and textures; the
build-time table).

**Cost unused.** 0 for an app without the crate (G1). Binary when
used: the `gltf` crate with its JSON model is the main cost; not
measured. The size is in the row's PR as the method says. Images
decode through the existing `png`/image route, which the web page
already carries.

### 3.9 Skinned and morph animation

**What it takes.** `joints` (4 x u16) and `weights` (4 x f32)
attributes (3.1); `NodeKind::SkinnedMesh { geometry, material, skin }`
where a `Skin` is a list of joint `NodeId`s and inverse bind matrices;
a storage buffer of joint matrices per skinned node written when any
joint's world version changes (joints are ordinary group nodes, so a
joint move is the existing one-node write); skinning in the vertex
stage (G4 chunk) reading that buffer. Morph targets: a storage buffer
of per-target deltas per geometry, uploaded once, and weights in the
node's slot. Clips: a crate `vapors-scene3d-anim` (G1) that samples
keyframes (glTF's step, linear, cubic spline) at `SurfaceFrame::now`
in a frame task and writes the animated nodes' transforms in one
batched call (3.11); its cost is O(animated channels) per frame, flat
in scene size, and it holds an animation request guard (particles
P6) while a clip plays, so a scene with no clip playing draws
nothing. Blend trees and state machines are not in this document.
About 700 lines across four PRs.

**Cost unused.** The skinned pipeline variant only when `SceneUses`
names a skinned mesh (G3); the clips crate only when depended on (G1).
Binary: not measured; no dependency beyond `vapors-gltf` for the data.

### 3.10 PBR and image-based lighting

**What it takes.** `MaterialKind::Pbr` with the glTF metallic-roughness
model (base colour, metallic, roughness, normal, occlusion and emissive
textures; `tangent` attribute or derivatives), the lighting chunk
(Cook-Torrance with GGX, as Bevy's `pbr_functions.wgsl` and glTF's
sample viewer implement it), and image-based lighting from an
environment cube map: a prefiltered specular mip chain and an
irradiance map computed by compute passes once per environment
(particles C1's infrastructure). Without IBL a PBR surface under one
light reads as dull; with it the scene renders to an HDR intermediate
(`Rgba16Float`) and tone-maps into the backing (3.11), so this row
brings the HDR path. Environment maps are a `TextureId` (3.1) loaded
as an equirectangular image and converted by a compute pass. About
900 lines across four PRs (material and chunk; environment
conversion; HDR intermediate and tone map; the glTF materials route).

**Cost unused.** `pbr` feature of `vapors-scene3d-gpu` (G2) and made
only when `SceneUses` names it (G3). Binary: the shader chunks are
tens of KB of WGSL as text (Bevy's `pbr.wgsl` family is about 100 KB
of source; vapors' would be smaller, not measured); the Rust side is
small. The HDR intermediate doubles the surface's colour memory while
in use.

### 3.11 Post-processing

**What it takes.** A `<Scene post={[Pass::Tonemap(..), Pass::Bloom(..),
Pass::Fxaa]}>` prop, a closed enum list so the compiler sees it (G3),
run as full-screen passes between the scene pass and the backing: tone
mapping first (needed by 3.10; Khronos PBR neutral and ACES as the
two choices), bloom (a mip chain with compute or render passes), FXAA
(the backing is single-sampled; MSAA is the alternative, one-scene S3b
decides the 2D rule). SSAO, TAA, depth of field and motion blur are
not in this document's slices: none has a use in vapors' apps and each
is its own measured row later. About 500 lines across three PRs.

**Cost unused.** `post` feature (G2); no intermediate texture and no
pass when the list is empty (G3). Each pass's GPU time is a `GpuTimer`
reading of its own, so the PR records what each costs at 1100 x 720 on
the UHD 630 and on the Pixel 9 Pro.

### 3.12 Physics hooks and ordered frame tasks

**What it takes.** No physics engine (the brief: "keep physics in
application Rust"). Two hooks make an engine such as rapier or avian
usable as an app dependency with flat per-frame cost:

- `Scene::set_transforms(&[(NodeId, Transform)])`, one call that marks
  each node once and recomputes descendants once (today's
  `set_transform` per node does the same work N times through the
  same `update`; the batch is an API shape, not a new algorithm), with
  the one-writer rule of design 09: a node whose transform is bound by
  a prop is refused in the batch naming the node and the component.
- Ordered frame tasks on the scene (the brief's slice 5; Threlte's
  `useTask` in shape, with named stages before/after): simulation,
  then camera, then resource updates, then render; each task gets
  `SurfaceFrame::now` and the step since the last frame; a task holds
  an animation guard while it runs (particles P6). A fixed-step
  accumulator is the task's own business in the first version (E8).
- A collision shape from a geometry's AABB (3.5) for the common case.

About 250 lines in two PRs.

**Cost unused.** 0: no tasks, no work; the batch call is the same code
path as one write. Binary: no dependency.

### 3.13 Level of detail

**What it takes.** `<Lod>` group whose children each carry `range={
(near, far)}` in camera distance (Bevy's `VisibilityRange`, Godot's
visibility ranges); one child shown per distance, the choice kept per
G5 and re-made for that node when the camera or the node moves.
Crossfade is a later row (it needs 3.7's blend). No automatic mesh
simplification: Godot's comes from meshoptimizer, a C library, and a
pure-Rust simplifier is a project of its own; glTF's `MSFT_lod` or
the author's own levels supply the meshes. About 200 lines.

**Cost unused.** 0 for a scene without `<Lod>` (G3); the choice is one
distance compare per Lod node per camera change.

### 3.14 Asset streaming

**What it takes.** Loading off the frame: `Meshes::geometry` validates
O(vertices) on the calling thread, so a large mesh validated inside a
frame breaks the 16.7 ms budget; the loader (3.8) validates and uploads
in chunks across frames with a per-frame byte budget (G6, a fixed
number in the refusal, measured on the Pixel 9 Pro), counted in
`FrameCounts`, and a node whose geometry is not yet uploaded is not
drawn until it is, with a counter (`nodes_pending`) so the state is
visible and a test can pin it, never silently skipped. Priority by
distance to the camera uses 3.5's boxes. Texture streaming by mip
level (loading small mips first) is a later row; Bevy and Godot do
not have it either. About 300 lines in two PRs.

**Cost unused.** 0 for a scene whose assets are given up front; the
chunked path is the only loader path, so there is one code path to
test (G4's spirit: one answer).

### 3.15 Audio hooks

**What it takes.** Audio is a platform service (design 07's area), not
a 3D feature: a crate `vapors-audio` (G1) over the Web Audio API on
the web and a native backend (cpal or rodio, its `unsafe` its own;
`cargo xtask deny` and the licence list decide), with play, stop, gain
and pan, driven from app code. The 3D hook is small: the active
camera's world transform as the listener and any node's world
transform as an emitter, read by the app each frame it has sounds
playing, and a distance attenuation helper. No bus system, no effects,
no spatialisation beyond pan and gain in this document. About 400
lines in three PRs, the platform part first.

**Cost unused.** 0 (G1). Binary when used: the backend crate; on the
web, `web-sys` features for `AudioContext` only. Not measured.

## 4. What is left out, and why

| Left out | Why |
|---|---|
| An ECS and a systems scheduler (Bevy's model) | Design 09 ("Deliberately left out"): the frame has fixed phases; scene data is flat arrays written by prop effects and tasks. A scheduler is a second answer to ordering that the analysis cannot see. |
| A physics engine, an audio engine with buses and effects | No vapors app needs either; both exist as crates an app can depend on, with 3.12 and 3.15 as the hooks. Bevy ships neither physics nor more than playback. |
| Order-independent transparency | Bevy 0.15's OIT costs a per-pixel linked list or layers of memory per frame whether or not any pixel overlaps; sorted blending (3.7) covers the apps' uses. Godot does without it. |
| Occlusion culling (Godot's Embree), portals | A C dependency vapors cannot wrap without `unsafe`, and the win needs a scene denser than any vapors app has. Frustum culling and LOD first; a GPU occlusion pass (depth pyramid in compute) is a possible later row with the particles plan's compute infrastructure, after a measurement shows overdraw. |
| Automatic mesh simplification at import | meshoptimizer is C; a pure-Rust simplifier is a project on its own. Authors supply LOD levels (3.13). |
| Cascaded, point and spot shadows; contact shadows | Design 09 caps shadows at one directional map; each further kind is a pass and a texture per light, and no app asks. The slot in the light enum and the shadow chunk stay open to them. |
| Global illumination (SDFGI, VoxelGI, light probes), reflection probes | Memory and passes that run whether or not the scene changes, against property 6; no use. IBL (3.10) gives one environment's indirect light at one texture's cost. |
| TAA, SSAO, SSR, depth of field, motion blur | Each is a measured row of its own after tone mapping, bloom and FXAA (3.11), none with a use yet. TAA needs motion vectors in every pipeline, a cost on every draw. |
| Virtual geometry (Bevy's experimental meshlets), ray tracing | Experimental in Bevy; no WebGPU ray tracing. |
| A render graph with user passes | One-scene R2's application material and 3.11's pass enum are the extension points; a general graph is a second renderer API the analysis cannot see. |
| Animation blend trees and state machines | After clips (3.9) play; no use yet. |
| Texture streaming by mip level | Neither engine has it; a later row of 3.14. |
| Networking, scripting, an editor, scene file formats of vapors' own | glTF is the interchange format; `.wisp` is the scene description; there is no second one. |
| WebGL fallback | WebGPU only, as everywhere (`docs/web.md`). |

## 5. Ranked order and slices

Ranked by: what vapors' own apps need next (touché's layer view,
design 09 T3; the racer), what each row unblocks, and cost. The game
path (Tom's direction) stays open at each step: nothing here closes a
row in section 3, and the order front-loads the rows every game needs
(materials, instancing, lighting, culling, picking, glTF).

1. Materials and textures (3.1)
2. Instancing (3.2)
3. Lit material and lights (3.3)
4. Frustum culling (3.5)
5. Picking (3.6)
6. Physics hooks and ordered tasks (3.12)
7. Transparency (3.7)
8. glTF (3.8)
9. Shadows (3.4)
10. Skinned and morph animation (3.9)
11. PBR and IBL, with the HDR intermediate and tone mapping (3.10, the first pass of 3.11)
12. Post-processing: bloom and FXAA (3.11)
13. LOD (3.13)
14. Asset streaming (3.14)
15. Audio hooks (3.15)

Each PR is under about 400 lines (estimate), built by a fresh agent,
each check proven by breaking what it guards, and lands after the
finish line, 3D PRs 4 to 5, and the one-scene and particles rows it
names. After each row the next is checked against what it found.
Every GPU PR pins `FrameCounts::surfaces` for an idle frame and for
one change on a scene that does not use the feature, equal to the
numbers before it; that pin is not repeated below.

| PR | Content | Accepted when | Depends on |
|---|---|---|---|
| M1 | `Materials`, `MaterialId`, `MaterialKind::Unlit { base }`; `NodeKind::Mesh { geometry, material }`; CSS colour multiplies `base` (4b's route); `<Mesh material={..}>` | #252's pins hold (80 bytes per change); a foreign `MaterialId` refused at the frame naming node and id; a material change writes one slot | 3D PR 4b, PR 5 |
| M2 | `Geometry` attributes `uv0`, `normal`, `colour`, validated per attribute (count equal to positions, finite); the vertex layout per attribute set; `Textures` with `TextureId` from the image pipeline or an app texture; `Unlit { texture }` | Pixel test: a textured plane shows the image at the right orientation under both projections; a material reading `uv0` on a geometry without it refused naming all three; `SceneUses` with one kind makes one pipeline (pipelines counted at mount) | M1 |
| M3 | `SceneUses` emitted by the compiler per crate from the templates' node and material kinds; the renderer makes its pipelines at mount from it; a node outside it refused naming the template | Snapshot test of the emitted value; a `<Mesh>` added from Rust with a kind the crate's templates never name is refused; pipelines made before the first frame (one-scene D7) | M2, build-time analysis (finish-line item 7) |
| I1 | `NodeKind::Instances`, the kept instance buffer on `Retained` (#247), one draw per group; under E1(a) plain meshes become instances of one and `slots.rs` is retired | 541 trees as one group: 1 draw; moving one tree writes 80 bytes; pixel test of placement | M1, IU4 PR 2a |
| I2 | `<Instances>` and keyed `<Instance transform>` under `{for}`: insert, remove, reorder, one change | Identity kept across reorder (the same row); removing one writes nothing for the others; geometry uploaded once across all four operations (bytes pinned) | I1, 3D PR 5 |
| L1 | `NodeKind::Light { Directional }`, the lights uniform, the 8-light limit refused | A light move writes one scene uniform, no node slots; 9 lights refused naming the limit | M3 |
| L2 | `MaterialKind::Lit`, the lighting chunk (G4), sRGB to linear and back; `<Light>` component; ambient on the scene root | Pixel test: a cube lit from one side, the lit face's value within 1 per channel of the CPU formula; a lit material with no light refused at mount | L1 |
| C1 | AABB per geometry and per node, `nodes_culled`, the kept visible set per G5 | An off-screen cube is not drawn and `nodes_culled` is 1; moving it on screen draws it; a camera move re-tests every node once (counted); idle re-tests 0 | M3 |
| C2 | Per-group culling for instances; the culling cost at 1,000 and 10,000 nodes in `vapors-perf` | The counts recorded in `docs/perf/`; E5 answered with a number | C1, I1 |
| K1 | The ray through the pointer against the visible set's AABBs; `pointerdown`/`up`/`click` on the hit node's anchor, bubbling; the scene skipped when no anchor listens | A click at a known projected position targets that mesh in both projections; a scene with no listeners costs 0 per pointer move (counted) | C1, 3D PR 4b |
| K2 | The triangle stage; `pointerenter`/`pointerleave` on anchors (owed from 3d-1c PR 5) | A click in the box but off the triangles misses; a leave fires when the pointer leaves the mesh and the window | K1 |
| T1 | Ordered frame tasks on the scene with `now` and the step; animation guards held by tasks | Two tasks run in the declared order every frame; a task stopping returns the app to `Next::Wait`; a cycle between tasks refused naming both | 3D PR 5, particles S1 and P6 |
| T2 | `Scene::set_transforms` batch with the one-writer refusal; an example using rapier or avian as an app dependency (`cargo xtask deny` passing) | 1,000 nodes moved in one call recompute each world once (counted); a bound node in the batch refused naming the component | T1 |
| A1 | `AlphaMode::{Opaque, Mask, Blend}`; blend nodes drawn after opaque, depth write off, sorted and kept per G5 | Pixel test: two overlapping translucent quads in both orders give the back-to-front result; sort re-run only on a camera or blend-node move (counted) | M3 |
| F1 | `vapors-gltf`: a `.glb`'s meshes with attributes and its node tree into the stores, every refusal naming the file and node; the brotli size with and without the crate in `docs/web.md` | A committed sample loads and matches a reference render; an unsupported extension refused naming it; a file with an index out of range refused naming the accessor | M2 |
| F2 | glTF materials (unlit and lit) and textures; the web page's fetch route with `await` | The sample's materials render through M2's and L2's pipelines; a missing image refused naming the URI | F1, L2 |
| F3 | The build-time table: a bundled `.glb` packed by the compiler per crate (E4) | A bad bundled file fails the build naming it; the loaded geometry is a constant (no validation at run time, counted 0) | F1, one-scene S5a |
| S1 | The `shadows` feature: the depth-only pipeline, the shadow pass, the light's fitted projection, kept per G5 | A cube's shadow lands on the plane where the CPU projection puts it, within one pixel; no shadow pass on a frame where nothing moved (draws pinned) | L2 |
| S2 | Shadow sampling in the lit chunk with a comparison sampler and a bias; the Pixel 9 Pro's cost recorded | Pixel test of a soft edge; acne absent at the test's grazing angle (a pixel rule stated in the test) | S1 |
| N1 | `joints` and `weights` attributes, `Skin`, the joint matrix storage buffer, the skinning chunk | A two-bone mesh bent at a known angle matches a CPU-skinned reference; a joint move writes that skin's buffer alone | F1 |
| N2 | Morph targets: delta buffers and weights in the slot | A target at weight 1 matches the target's positions; weight 0.5 the midpoint | N1 |
| N3 | `vapors-scene3d-anim`: clip sampling (step, linear, cubic) in a task, writing the batch; the guard while playing | A clip's pose at t matches glTF's sample viewer's interpolation; after the clip ends, 0 frames idle | N1, T1, T2 |
| N4 | glTF skins, morphs and animations into the stores | The rigged sample plays; its per-frame bytes are the joint buffers alone (pinned) | N3, F2 |
| P1 | `MaterialKind::Pbr` with the metallic-roughness chunk under direct lights, the `pbr` feature | A sphere grid (metallic by roughness) matches glTF's sample viewer within a stated tolerance at four points | L2, M2 |
| P2 | The HDR intermediate (`Rgba16Float`) and tone mapping (PBR neutral, ACES) as the first post pass, made only when `SceneUses` or the pass list needs it | An unlit scene still renders straight to the backing (no intermediate, texture memory pinned); the lit scene tone-mapped matches a reference at four values | P1 |
| P3 | Environment maps: equirectangular to cube, prefiltered specular and irradiance by compute; IBL in the PBR chunk | The sphere grid under an environment matches the sample viewer's within tolerance; the conversion runs once per environment (dispatches pinned) | P2, particles C1 |
| P4 | glTF PBR materials and textures (normal, occlusion, emissive) | The damaged-helmet sample renders within tolerance of the sample viewer | P3, F2 |
| Q1 | Bloom and FXAA passes; each pass's GPU time on the UHD 630 and the Pixel 9 Pro in `docs/perf/` | An empty pass list adds no pass (counted); bloom's mip chain is the stated count; FXAA's edge matches the reference within the pixel rule | P2 |
| D1 | `<Lod>` with ranges, kept per G5 | The near child shown at 1 unit and the far at 100; one re-choice per camera move (counted) | C1 |
| R1 | Chunked validation and upload with a per-frame byte budget, `nodes_pending`; priority by distance | A 1 M-vertex mesh loads over N frames with no frame over the budget (frame-to-idle measured as one-scene P0 does); the node drawn only once complete | F1, C1 |
| R2 | The web route: fetch, chunked decode in the page's idle frames | The same on SwiftShader through the page contract; 0 frames after the load completes | R1 |
| U1 | `vapors-audio`: Web Audio and a native backend, play/stop/gain/pan; the licence and deny checks | A tone plays natively and on the page (a page-contract case reads `audio: ok`); no backend linked without the crate (size recorded) | none |
| U2 | Listener and emitter transforms from the scene with distance attenuation | A sound's gain at two distances matches the stated curve | U1, 3D PR 5 |

33 PRs. Critical path to a lit, culled, pickable, glTF-loaded scene
(the point at which a small game is writable): M1, M2, M3, L1, L2,
C1, K1, F1, F2, then T1, T2 for simulation. Instancing (I1, I2),
transparency (A1) and shadows (S1, S2) hang off M3 and L2 in parallel.

## 6. Decisions for Tom

Sections 3 and 5 follow each recommendation; where another option
would change a row, the decision says which.

**E1. Meshes as instances of one.** (a) One instanced pipeline: every
mesh node is an `Instances` group of one, its world matrix and colour
in a vertex-stepped buffer on `Retained`; `slots.rs` and its dynamic
offsets are retired, so there is one draw path and one write path for
meshes and instances. (b) Keep the uniform slots for meshes and add a
second pipeline for instances. (a) changes #252's pinned 80 bytes to
80 bytes in a different buffer (the number holds) and removes about
240 lines (`slots.rs`, `slot_table.rs`); (b) keeps today's code and
adds a path. Recommendation: (a), in I1: one answer to "where is a
mesh's matrix", and the shader chunk for lighting and skinning is
written once.

**E2. How shaders are composed.** (a) WGSL chunks concatenated by
vapors at pipeline creation, keyed by material kind and attribute set,
with `override` constants for the small choices, and a unit test that
every combination `SceneUses` can name compiles under naga (the
`gpu-harness` has the device). (b) A preprocessor crate (naga_oil, as
Bevy uses) with `#import` and `#ifdef`: a dependency and a second
language in the shaders. (c) One über-shader with uniform branches:
every draw pays every branch. Recommendation: (a): the combinations
are few (unlit, lit, PBR, times textured, skinned, shadowed) and
known at build time (G3).

**E3. Which lighting model first.** (a) Lit (Lambert and Blinn-Phong,
one directional light, no textures required) in L2, PBR later in P1.
(b) PBR directly, skipping Lit. (a) is the brief's MVP and gives the
racer and touché's layer view lighting without the HDR path; (b)
saves one material kind but brings P2's HDR intermediate forward and
reads flat without IBL (P3). Recommendation: (a).

**E4. glTF at build time, at run time, or both.** (a) Both: the
run-time loader first (G1, G2), then the compiler packs bundled files
into a static table (G3) so a bad asset fails the build and the
geometry is a constant to the analysis. (b) Run time only: simpler,
but a bundled model is validated on every start, and the analysis
sees a known variable where a constant was possible. (c) Build time
only: user content (a model dropped on the page) could not load.
Recommendation: (a), run time first because it is the route both
need and the sample files test it.

**E5. When culling and picking get a bounding hierarchy.** (a) A flat
scan of kept boxes (C1, K1) until C2's measurement at 1,000 and
10,000 nodes shows the camera-move re-test over a stated budget (a
tenth of the 16.7 ms frame, say), then a BVH rebuilt incrementally on
node moves. (b) A BVH from the start. Recommendation: (a): the scan
is O(nodes) only on a camera move, idle is 0, and the number decides.

**E6. The HDR intermediate.** (a) Chosen per scene from `SceneUses`
and the pass list: `Rgba16Float` and a tone-map pass only when a PBR
material or a pass needs it; otherwise the scene renders straight to
the `Rgba8Unorm` backing as today. (b) Always HDR once the `pbr`
feature is on: simpler, but every scene pays the intermediate's
memory and the extra pass. Recommendation: (a).

**E7. How features are gated.** (a) Separate crates for anything with
a dependency or a backend (`vapors-gltf`, `vapors-scene3d-anim`,
`vapors-audio`), cargo features of `vapors-scene3d-gpu` for pipeline
variants with no dependency (`shadows`, `pbr`, `post`), each refusing
by name when off, and `SceneUses` deciding at mount what is made.
(b) Features only, in one renderer crate: fewer crates, but the
dependencies of an unused feature still resolve in `Cargo.lock` and
`cargo xtask deny`. (c) Crates only: a crate per pipeline variant is
a crate boundary through the renderer's pass. Recommendation: (a).

**E8. A fixed-step tick in the scheduler.** (a) Not yet: tasks get
`now` and the step (T1); a fixed-step accumulator is the physics
task's own, and the racer's model stays as it is. (b) A `fixed_step`
stage in the scene's task order with a stated rate, so every
simulation in vapors runs the same way and the perf harness can step
it. Recommendation: (a) now, (b) when a second simulation exists; T1's
stage names leave room for it.

**E9. Textures: the engine's image pipeline or the app's.** (a) Both,
through one `TextureId`: images decoded by the painter's route
(`crates/renderer/src/image.rs`, mips) and wgpu textures the app
owns (design 09: a layer's render result as a material), each
refused when its format is not the material's expected one. (b) The
image pipeline only. Recommendation: (a): touché's layer view needs
(a)'s second route.

**E10. Where this plan sits in the order.** (a) After the finish line
and the one-scene and particles rows it names, as section 0 says. (b)
M1 to M3 (materials, `SceneUses`) pulled forward beside one-scene's
R1 and R2, since R2's application material is a material kind and
R1's shade is an attribute, and building them on M1's store avoids a
second migration. Recommendation: (b) for M1 and M2 only if the lead
finds R1/R2 would otherwise build a `colour`-only path that M1
rewrites; otherwise (a). This is the one place this document touches
current work, and only as a question.

## 7. Risks

- **The nested-component compiler gap** (#251, "Found"): until a
  generated `.wisp` component can sit inside another's children, no
  user-written `<Car>` goes under `<Group>`, which every row's
  example scene wants. One-scene R3a1 owes the fix; this plan assumes
  it lands first.
- **`SceneUses` and sheets from Rust.** G3 knows only what a crate's
  templates name. A node added from Rust (the racer's `Scene::add`
  today) is outside the analysis; M3 refuses it unless the app
  declares the kinds it will add, which is a second source of truth.
  The build-time styles plan faces the same question for stylesheets
  added from Rust (`docs/plans/build-time-styles.md`, "What it knows")
  and its answer should be reused.
- **Size numbers are all unmeasured.** Each row's PR measures its own
  brotli delta by the stated method; if `vapors-gltf` or the audio
  backend costs more than touché's page budget allows
  (`check_dist`, about 1.7 MB), the crate stays out of touché's build
  (G1 makes that the default) and the number is published.
- **WebGPU limits on the Pixel 9 Pro and SwiftShader.** Storage
  buffers in the vertex stage (skinning, N1) and compute for IBL (P3)
  are unverified on both (`particles.md`, risk 1); each row's web
  check runs before the row is accepted, and a device that cannot is
  refused naming the limit, never degraded.
- **Pipeline creation at mount.** G3 makes every pipeline a scene can
  need at mount; a scene naming six kinds makes six pipelines before
  its first frame. On ANV that may exceed a frame (one-scene D7's
  risk); the count is pinned and the time measured in M3.
- **Depth attachment discarded** (`draw.rs:92`): shadows (S1), blend
  (A1) and picking's depth readback, if ever used, all draw or read in
  the scene pass; a later pass needing depth must store it, which
  costs a resolve on tiled GPUs. Each such row says which it does.
- **Two engines' details may have moved.** The Bevy and Godot columns
  in section 2 are from their documentation as known here, not fetched
  today; a row that copies a behaviour checks the current docs first.
- **Scope.** 33 PRs after a finish line that is itself open. The
  ranking exists so that stopping after any row leaves something
  working; nothing past M3 is a prerequisite for the rows before it.
