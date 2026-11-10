<!-- Source: untracked GPU_SURFACE_3D_BRIEF.md at the vapors main checkout root, written 2026-11-09; copied unchanged 2026-11-10. -->

# Brief: composable GPU surfaces and declarative 3D components

Status: approved direction; proposed API names and implementation details remain
subject to review. Written 2026-11-09.

## Objective

Add a GPU-content surface that participates in ordinary Vapors layout and
composition, then build a small declarative 3D component system on it. The first
surface must work inside panels as well as filling the application window.

The surface is useful independently of 3D: Touché should be able to host its
image renderer through the same boundary. The racer supplies a working consumer
for validating 3D composition, animation and touch interaction.

The user approved a Threlte-like declarative authoring model. This means nested
components, reactive properties, reusable scene parts and keyed instance loops.
It does not imply Three.js compatibility or a general-purpose game engine.

## Existing code and coordination

The main clone is `/home/tom/projects/vapors`. The racer work is isolated in
`/home/tom/projects/vapors-racer`, branch `demo/top-down-racer`, draft PR:
https://github.com/tomyan/vapors/pull/132

The component-driven racer revision is `f25556a`. Inspect the current branches
before implementation; do not assume the main clone already contains this work.
Coordinate integration with other active work rather than overwriting a checkout.

Useful references on the racer branch:

- `crates/racer/ui/hud.wisp`: composition of RaceStats, RaceMenu, CameraToggle,
  DrivingControls and CarSetup through shared signals.
- `crates/components/src/hold_button.wisp`: reusable captured press-and-hold input.
- `crates/racer/src/gpu.rs`, `gpu/`, `track.wgsl`: current scene rendering.
- `crates/racer/src/model/camera.rs`: smooth Fixed / Follow camera behavior.
- `crates/app/src/painter.rs`: ScenePainter draws compiled UI over app content.
- `crates/scene/src/display.rs`: current UI display list; keep its layer boundaries
  in mind when introducing a GPU-content reference.
- `docs/components.md`, `docs/hold-button.md`, `docs/RACER_PLAN.md`.

Today, the racer owns its GPU pipelines and renders first, then overlays the
compiled HUD. Its scene is not a template component. Existing component syntax,
signals, loops and lifecycle handling are reusable foundations; the surface and
3D scene integration are new work.

## Layer 1: GPU surface

Introduce an ordinary layout element backed by a custom renderer. Illustrative
API only:

```html
<GpuSurface
    renderer={renderer}
    render_mode={RenderMode::OnDemand}
    resolution_scale={1.0}
/>
```

The renderer does not own a window, event loop or browser canvas. The surface
must not require a camera or assume its content is 3D.

Required contract:

- Supply logical content bounds, physical backing dimensions and scale factor.
- Use the application's shared GPU device and queue. Give the renderer explicit
  target-format and command-encoding access. Vapors controls submission order.
- Define mount, resize, render and teardown responsibilities. Allocate persistent
  resources once and resize backing targets only when their size changes.
- Participate in normal paint order and clipping. Sibling UI can appear above it;
  the surface cannot overwrite unrelated panels.
- Route input in surface-local coordinates. Use existing focus and pointer-capture
  semantics. UI above the surface receives input before the surface beneath it.
- Default to on-demand rendering. Explicit animation requests keep frames running;
  ending animation returns to idle. Coalesce requests from multiple surfaces.
- Treat zero-sized/hidden surfaces as inactive, not as GPU errors. Refuse invalid
  dimensions or unsupported configurations with actionable errors.
- Define colour space, alpha convention and texture format explicitly. Keep the
  contract suitable for a later colour-managed photo renderer; do not bake a 3D
  tone mapper into the generic surface.
- Report GPU loss and renderer failure visibly. Do not silently substitute blank
  output or claim recovery. Automatic recovery is outside this MVP.

Recommended first implementation: render each surface into an offscreen GPU
texture, then compose that texture through Vapors in paint order. No CPU
readback. Account for the extra texture memory and composition pass. A direct
window-target path can be a later measured optimization without changing the
component API.

Write down the first release's supported clipping and composition subset. It
must include rectangular clipping, panel resizing and overlapping ordinary UI.
Unsupported cases must be rejected explicitly, not approximated silently.

## Layer 2: declarative scene

`Scene3d` wraps the surface and establishes scene context and ownership. A scene
node participates in 3D transforms and rendering, not CSS layout or UI hit-testing.
CSS lays out the surface; it does not lay out each tree.

Illustrative composition, not currently supported syntax/API:

```html
<Scene3d camera={active_camera} render_mode={RenderMode::OnDemand}>
    <OrthographicCamera
        handle={overview_camera}
        position={overview_position.get()}
        target={camera_target.get()}
    />
    <PerspectiveCamera
        handle={follow_camera}
        position={follow_position.get()}
        target={camera_target.get()}
        vertical_fov={Degrees(50.0)}
    />
    <DirectionalLight direction={sun_direction} />
    <Circuit />
    <Group transform={car_transform.get()}><Car /></Group>
    <Instances geometry={tree_mesh} material={tree_material}>
        {for tree in trees.get() key=tree.id}
            <Instance transform={tree.transform} />
        {/for}
    </Instances>
</Scene3d>
```

Keep the HUD as normal Vapors components above the scene.

MVP primitives:

- Scene root and explicit active camera handle.
- Group with hierarchical translation, rotation and scale.
- Mesh referencing shared geometry and material handles.
- Orthographic and perspective cameras.
- Opaque unlit and basic-lit materials, with one directional light.
- Explicit instancing of shared geometry/material, with stable keyed instances.
- Ordered frame tasks with start/stop and component-owned cleanup.

Provide a small geometry set sufficient for an example, plus an application mesh
upload API. Existing procedural racer geometry can use that API. Do not make
asset loading a prerequisite for proving composition.

Define coordinate conventions, units, handedness, angle types and matrix order
once. Validate camera projection parameters, transforms, mesh buffers and resource
handles. Missing required resources and invalid references must produce explicit
errors identifying the relevant component where possible.

There must be one answer to which camera is active. An unresolved active-camera
handle is an error; do not pick whichever camera happened to mount first.

## Reactivity, lifecycle and performance

Mount creates persistent scene nodes. Signal changes update affected properties;
ordinary frame rendering does not rebuild the component tree or mesh buffers.
Keyed loops preserve node/instance identity across reorder and edits.

Share geometry and materials through explicit handles. Unmount releases a
component's ownership without destroying resources still used by other nodes.
Define disposal and stale-handle checks, including device loss. Avoid ownership
cycles between components, scene nodes and GPU resources.

`Instances` is an explicit batching contract. Updating one transform should not
re-upload immutable geometry. Removing one tree must not rebuild other tree
meshes. Start with a simple instance-buffer strategy; measure upload volume and
draw count before adding complexity.

Frame tasks need deterministic ordering: simulation, then camera, then resource
updates and rendering. Signal changes and running tasks invalidate the surface.
Tasks stop on unmount; invalid dependencies or cycles are errors. Reuse the
existing window scheduler rather than creating an independent animation loop for
each component.

Do not claim a performance improvement from declarative syntax alone. Compare
frame-time distributions, CPU update time, draw calls, upload bytes, texture memory
and idle rendering against the existing racer using matched settings and devices.

## Implementation slices and acceptance checks

1. **Panel-capable GPU surface.** Mount the existing racer renderer through the
   surface API, without changing its scene model. Prove resize, device scale,
   rectangular clipping, UI overlays, input coordinates, two independent panels,
   unmount and idle rendering. Include a non-3D content example to check that the
   API does not require scene concepts.

2. **Persistent scene and camera.** Show a mesh through Group/Mesh and both camera
   projections. Prove parent transforms, active-camera validation, invalid-input
   refusal and lifecycle cleanup. Compare ground and object projection so their
   transforms cannot diverge unnoticed.

3. **Reusable scene components.** Express Car and Tree as component compositions
   using shared resources and basic lighting. Keep physics in application Rust.
   Check material/geometry sharing, mount/unmount and depth occlusion.

4. **Keyed GPU instances.** Express the tree population with a template loop under
   Instances. Test insert, remove, reorder and one-instance transform changes.
   Assert stable identity, resource lifetime and unchanged geometry-upload counts.

5. **Animation and integration.** Drive the existing racer and Fixed / Follow
   camera through ordered tasks. Verify smooth switching, pause, focus loss,
   cancellation and return to idle. Preserve current HUD and touch behavior.

Each slice should be independently demonstrable before the next starts. Put the
agreed implementation design and slice status in a design document. Follow the
repository's red/green/refactor workflow: prove new checks fail, implement, review,
run required checks, commit and push. Reassess the next slice after each result.
If a discovered limitation requires changing the agreed architecture, stop and
present the choices to the user.

Retain the racer browser regressions for camera transitions, touch capture outside
controls, independent release, typed tuning, viewport changes and short landscape.
Add actual-device checks before making claims about Android stability or speed.

## Deferred work

No glTF pipeline, skeletal animation, physics engine, shadow maps, transparent
material sorting, mesh-level picking, post-processing stack or automatic GPU-loss
recovery in this MVP. Surface-level input is sufficient for driving and an initial
orbit-camera example. Preserve an extension point for application rendering where
needed, without making a complete rendering engine part of the generic surface.

## Threlte references

Use these as authoring and scheduling references, not compatibility promises:

- Canvas context, layout-driven sizing and render modes:
  https://threlte.xyz/docs/reference/core/canvas/
- Declarative object composition and ownership:
  https://threlte.xyz/docs/reference/core/t/
- Ordered tasks and invalidation:
  https://threlte.xyz/docs/reference/core/use-task/

Unlike Threlte's Three.js object wrappers, this proposal uses typed Rust scene
objects and wgpu resources. The useful similarity is component composition and
reactive updates; renderer APIs, resource types and supported features differ.
