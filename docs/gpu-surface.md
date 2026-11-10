# GPU surfaces

A `gpu-surface` element is a box in ordinary layout whose content an
application renderer draws on the GPU. It is layer 1 of the 3D lane
(touche decision `2026-11-09-3d-surface-first.md`); the declarative
scene (layer 2) is built on it. Nothing in it assumes 3D: a surface
needs no camera, depth or scene.

## Slice status

| Slice | Content | Status |
|---|---|---|
| 1a | `gpu-surface` element, `Item::Surface` in the display list, the refusals; `ScenePainter` refuses a frame with a surface (`PaintError::Surface`) until 1b | this PR |
| 1b | `SurfaceRenderer` and the painter's surface host: offscreen texture, resize, scale, clip, on-demand rendering, failures; a non-3D example | next |
| 1c | the racer's renderer mounted through the surface; two panels; surface-local input | after 1b |
| 2-5 | scene, cameras, components, instances, tasks | not started |

Touche's image renderer drawing through a surface in a panel comes
through a touche pin bump after 1c.

## The element

```html
<gpu-surface name="track" class="world"></gpu-surface>
```

- `name` names the renderer the application mounts on its painter. A
  `gpu-surface` without one, or with `name=""`, is refused. Two
  surfaces painted in one frame with the same name are refused by the
  painter (`PaintError::DuplicateName`, 1b); a hidden or `display: none`
  surface may share a name, so tabs can swap one renderer.
- CSS sizes it like a `div`; it has no natural size.
- The renderer draws over the content box, in device pixels, after the
  element's background and border and before its children: children
  and later siblings paint over it.
- `padding` other than zero (`0` and `0%` are zero) is refused, so the content box is the
  padding box and a pointer event's `offset` on the surface is in
  surface-local CSS pixels (see Input).
- `display: inline` is refused (inline-block content is not laid out
  yet). Each style refusal names the declaration's sheet and line. A
  hidden surface (`visibility: hidden`) paints no item.

## Display list

`Item::Surface { surface, rect }`: `surface` is a `SurfaceId` the
painter resolves through `Painted::surface`, to the element's `name`;
`rect` is the content box snapped to device pixels. Surfaces batch as
their own run kind, in painting order, under the clip set when they
were pushed.

## Supported composition (first release)

| Case | Behaviour |
|---|---|
| Ancestor `overflow: hidden` or `clip`, rectangular | Clipped: the composite draws under the run's scissor |
| UI painted later (siblings, children, positioned elements) | Drawn over the surface |
| Panel resize, device scale change | The backing texture is reallocated at the new device-pixel size (1b) |
| Rounded clip on an ancestor | Refused document-wide (`style/overflow.rs`) |
| `border-radius` on the surface | Refused |
| `opacity`, `transform`, filters | Refused document-wide (deferred properties) |
| Zero-sized surface | No item is painted; the renderer is not called |

## Input

Pointer events reach a surface through ordinary hit testing and
dispatch: UI over it is hit first, capture works as for any element.
On an event whose target is the surface, `Event::offset` is from the
content box's top-left in CSS pixels; multiply by the scale factor for
backing pixels.

## Renderer contract (1b, proposed)

To be written with 1b: shared device and queue, a command encoder vapors
submits, mount/resize/render/teardown, `Rgba8Unorm` holding sRGB-encoded
values with straight alpha (the format and convention `img` textures
use), on-demand rendering with explicit invalidation and animation
requests, and renderer failure and GPU loss reported, not substituted
with blank output.
