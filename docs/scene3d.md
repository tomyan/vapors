# The 3D scene

Layer 2 of the 3D lane: a scene of groups, meshes and cameras drawn on
a `gpu-surface` (layer 1, [gpu-surface.md](gpu-surface.md)). Plan:
`docs/plans/3d-2.md`; CSS on scene nodes: `docs/plans/css-for-3d.md`.

| Crate | What it holds |
|---|---|
| `vapors-scene3d` | the maths, the node store (`Scene`), the geometry store (`Meshes`); no GPU |
| `vapors-scene3d-gpu` | `SceneRenderer`, a `SurfaceRenderer` drawing one `Scene` |
| `vapors-scene3d-components` | `<Scene>`, `<Group>`, `<Mesh>`, `<Camera>` for `.wisp` templates |

## Slice status

| Row | Content | Status |
|---|---|---|
| 1 | `vapors-scene3d`: conventions, `Transform`, `Projection` | done |
| 2 | the scene store: generational ids, world matrices, the active camera | done |
| 3 | `SceneRenderer`: geometry upload, uniform slots, counts in `FrameCounts::surfaces` | done |
| 4 | the components, reactive transforms, one uniform per change | done |
| 4b | CSS on scene nodes: `css-for-3d.md` C1 to C4 | see that plan's table |
| 5 | lifecycle: keyed rows, the renderer's drop, geometry release; this doc | 5a in review; 5b, geometry handles, this branch |

## Conventions

The same in every crate (`vapors-scene3d`'s crate docs):

- Right-handed: +X right, +Y up, +Z towards the viewer; a camera looks
  down its own -Z.
- Units are the application's; the examples use metres.
- Angles are radians.
- Column vectors; a node's world matrix is `parent_world * local`; a
  `Transform`'s matrix is `translation * rotation * scale`.
- Clip depth 0 (near) to 1 (far), NDC +Y up, as in wgpu.

Every constructor refuses input it cannot use (`SceneError`), naming
the field; nothing is clamped.

## The scene in Rust

```rust
let scene = Rc::new(RefCell::new(Scene::new()));
let meshes = Rc::new(RefCell::new(Meshes::new()));
let square = meshes.borrow_mut().geometry(positions, indices, shades)?; // a GeometryHandle
let renderer = SceneRenderer::new(Rc::clone(&scene), Rc::clone(&meshes));
let surface = app.mount("view", Box::new(renderer))?;
```

- `Scene::add(parent, kind, local)` adds a group, a mesh
  (`NodeKind::Mesh { geometry, colour }`) or a camera; `remove` takes
  the subtree; `set_transform` and `reparent` move a node. A removed
  node's id is refused (`StoreError::Stale`), never another node's.
- The root names the active camera by `CameraHandle`; none, two, or an
  unresolved handle is refused (`vapors-scene3d`'s `camera_tests.rs`).
- `Meshes::geometry(positions, indices, shades)` takes a shade from 0
  to 1 for each position (one-scene D4, row R1): the vertex is drawn
  in that fraction of the node's colour (multiplied on the
  sRGB-encoded channels, as the racer's baked shades were; not linear
  light), so one CSS colour gives a
  shaded solid without lights or normals; the shade does not follow
  the camera. Not one shade per position, or one outside 0 to 1, is
  refused; there is no default. It validates triangles and returns a
  `GeometryHandle`: a counted claim, cloned to share and not `Copy`
  (lead decision (B), 2026-11-14). A mesh node holds a clone in
  `NodeKind::Mesh`, so a piece is held while the application keeps a
  handle or any mesh node names it. Once the last handle is dropped,
  `Meshes::release_unclaimed` releases it; each render runs that sweep
  and drops the renderer's buffers for every released piece
  (`RendererLog::made` and `freed` of `Resource::Geometry` count them). A
  released piece's `GeometryId` (from `GeometryHandle::id`) is stale:
  `Meshes::get` and `Meshes::holds` find nothing. No node can name a
  released piece, since each holds a handle; the frame's refusal of a
  piece its `Meshes` does not hold stays as the guard, naming a broken
  invariant. A handle from another `Meshes` is refused at the frame,
  naming the node. Release waits for the next render of a renderer
  sharing the `Meshes`: a page idle and drawing on demand keeps a
  released piece's data until it renders again. A render borrows the
  `Meshes` mutably for its sweep, so an application must not hold a
  borrow of it across a frame (the `RefCell` panics).

## What a frame does

`SceneRenderer::render` runs `Scene::update`, uploads each piece of
geometry a mesh names that this renderer has not uploaded (16 bytes a
vertex: position and shade), writes the
uniform slot (80 bytes: world matrix and colour) of each mesh whose
world version changed since this renderer wrote it, writes the camera
(64 bytes) when it changed, and draws each mesh, depth-tested. Every
upload, write and draw goes through `SurfaceFrame::work`, so
`FrameCounts::surfaces` holds them. An idle frame writes nothing; a
moved node writes its subtree's slots; a recoloured mesh writes its
own (`scene3d-gpu/tests/gpu_scene_counts.rs` pins the bytes).

## In a template

```html
<gpu-surface name="view" class="view"></gpu-surface>
<Scene scene={scene} surface={surface} camera={eye}>
  <Camera handle={eye} projection={lens} transform={back} />
  <Group transform={spin}>
    {for car in cars.get() key=car.id}
      <Mesh geometry={hull} transform={place(car)} />
    {/for}
  </Group>
</Scene>
```

- Each component adds its node at mount and keeps one anchor element
  (tag `scene`, `group`, `mesh` or `camera`) where its tag is; the
  anchor has `display: none` from the user-agent sheet, so no box, no
  paint and no place in the accessibility tree. Its children are
  placed in the anchor, and a node component finds its parent node
  from the anchor it is placed in, also when an `{if}` or `{for}` builds
  it later.
- `transform` is a signal: a change writes that node's uniform and its
  subtree's, uploads no geometry, and invalidates the surface.
- Required props (`scene`, `surface`, `geometry`, `handle`,
  `projection`) are `Option`s refused at mount when `None`. Markup that
  is not a node component inside `<Scene>` or `<Group>` is refused,
  naming its template site.

## Lifecycle

- A component's unmount (an `{if}` hiding it, a `{for}` dropping its
  row, `unmount`) removes its node and the node's subtree, with the
  node's geometry handle, and invalidates the surface. The page that
  passed `geometry={square.clone()}` still holds its own handle, so a
  mesh hidden and shown again finds its piece; once the page is
  unmounted and no other handle lives, the next render releases it.
- In a keyed `{for}`, a row keeps its node across a reorder: no node
  is removed or added. The scene's child order is not changed by a
  reorder (children keep insertion order; an opaque, depth-tested draw
  does not depend on it). A dropped row removes its node alone; an
  added row adds one (`scene3d-components/tests/scene_rows.rs`).
- Unmounting the surface (`ElementsApp::unmount`, then dropping what it
  returns) drops the `SceneRenderer` and frees every GPU resource it
  made: pipeline, depth view, uniform slots, camera uniform and each
  uploaded piece of geometry. `SceneRenderer::log` returns a
  `RendererLog` shared with the application: `dropped()` says the drop
  ran, and `made(kind)` and `freed(kind)` count each kind of resource
  (`Resource`); each resource carries a record dropped after it, so a
  leaked one stays made and never freed
  (`scene3d-components/tests/gpu_scene_unmount.rs`). Remove the
  `gpu-surface` element first: a document holding a surface with no
  renderer is refused at the frame.

## CSS

Scene anchors are styled by the cascade like any element; which
longhands each tag reads, what is refused, and how a mesh's colour and
visibility reach its node are in `docs/plans/css-for-3d.md` (C1 to
C3), and move here as C8.
