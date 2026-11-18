# Component events: listeners on component tags, and where a mesh was hit

Date: 2026-11-18. Decided by: Tom, with the lead. Closes the design
question in #506. Companion to `2026-11-18-style-variants.md`: styles
say how a component can look, events what it answers.

## Decision

1. **A component tag takes listeners, as an element does** (general,
   not only scene tags). `onclick={handler}` on `<Button>`, `<Mesh>` or
   `<Group>` attaches to the component's root element: the element its
   template puts first at top level, or for a scene component its
   anchor. Same handler signature as on an element, same bubbling, same
   `stop_propagation` and `set_pointer_capture` (`crates/dom/src/event.rs`).
   A component whose template has no single root element refuses
   listeners on its tag, positioned, naming the fix (wrap the template in
   one element).
2. **Which events a component takes is part of its API.** A component's
   events are those its root element can receive: pointer, wheel and
   click events for every component; keyboard, focus and input events
   only when the root can take focus or input. A scene component's
   anchor takes pointer, wheel and click events. A listener for an event
   the root cannot receive is refused, listing the ones it can.
3. **Face level and object level are where the handler goes.** Picking
   already makes the mesh under the pointer the target, and the event
   bubbles through the anchors (`<Mesh>` to `<Group>` to `<Scene>` to the
   `gpu-surface`), as `crates/scene3d-components/tests/scene_hits.rs`
   shows. A handler on a cube's `<Group>` hears every face; one on a
   face's `<Mesh>` hears that face, and can stop the event there.
4. **A scene pointer event says where the mesh was hit: `e.hit()`.**

   ```rust
   pub struct Hit {
       pub point: Vec3,         // world space
       pub local: Vec3,         // the mesh's own space
       pub normal: Vec3,        // surface normal at the hit, world space
       pub part: Option<Part>,  // the geometry's named part, when it has parts
   }
   ```

   `e.hit()` is `Option<Hit>`: `None` for an event whose target is not a
   scene node. Both ways of telling faces apart are supported:
   - **normals now**: the pick finds the triangle, so its normal is
     known; a box face is the axis of the normal's largest component;
   - **typed parts**: a geometry builder may mark triangle ranges as
     parts of a type it names (`CubePart::Top`), and the hit reports the
     part. Typed as variants are, never an index or a string.
5. **Ergonomics are part of the feature**, as for style variants:
   `wisp-check` knows which events each tag takes and refuses the rest
   without cargo, positioned in the `.wisp` with the valid list;
   `wisp-check --api` lists a component's events with its props and
   styles; `docs/components.md`, `docs/scene3d.md` and the plugin skill
   carry the example below.

## Example

```html
<Scene scene={scene} surface={surface}>
  <!-- object level: any face's click bubbles here -->
  <Group transform={cube_at(spins.get())} onclick={move |_| spins.update(|n| *n += 1)}>
    {#for face in 0..6}
      <!-- face level: this face; stop it reaching the cube -->
      <Mesh geometry={faces[face].clone()} class:Hue={hues[face].get()}
            onclick={move |e| { hues[face].update(|h| *h = h.next()); e.stop_propagation(); }} />
    {/for}
  </Group>
  <!-- one mesh, six faces: which face from the hit -->
  <Mesh geometry={cube.clone()}
        onclick={move |e| if let Some(hit) = e.hit() { tapped.set(side(hit.normal)) }} />
  <!-- drag to turn -->
  <Group transform={turned.get()}
         onpointerdown={move |e| e.set_pointer_capture()}
         onpointermove={move |e| if e.pressed() { turn(&turned, e) }} />
</Scene>
```

Hover needs no handler: `:hover` matches a mesh's anchor, so a style
variant can light a face under the pointer.

## What changes where

- `crates/compiler`: listeners on component tags (today refused in
  `template/attributes.rs`: "a listener is for an element"), attached to
  the root; the refusals above; `wisp-check` and `--api`.
- `crates/scene3d`: `pick` returns the hit (distance, triangle, point,
  normal), not only the node; geometry parts with a typed id.
- `crates/scene3d-components`: the hit carried on the dispatched pointer
  event; `e.hit()`.

## Order

1. Run 1: listeners on component tags (general), with the refusals and
   `wisp-check`; scene tags then work through their anchors.
2. Run 2: `pick` returns the hit; `e.hit()` with point, local and normal.
3. Run 3: typed geometry parts and `hit.part`; `--api`; docs and skill.

## Open

- Whether a component may route a listener to an element other than
  its root (for example a labelled input whose root is a wrapper) is left
  until a component needs it; the root rule covers the cases seen.
