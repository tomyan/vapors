# GPU surfaces

A `gpu-surface` element is a box in ordinary layout whose content an
application renderer draws on the GPU. It is layer 1 of the 3D lane
(touche decision `2026-11-09-3d-surface-first.md`); the declarative
scene (layer 2) is built on it. Nothing in it assumes 3D: a surface
needs no camera, depth or scene.

## Slice status

| Slice | Content | Status |
|---|---|---|
| 1a | `gpu-surface` element, `Item::Surface` in the display list, the refusals | #139 |
| 1b | `SurfaceRenderer` and the painter's surface host: offscreen texture, resize, scale, clip, on-demand rendering, failures | #140 |
| 1b2 | `ElementsApp` mounting, `SurfaceHandle`, `Next` from `wants_frame`; the gradient example | this PR and the next |
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
  surfaces painted in one frame (in view or wholly clipped) with the
  same name are refused by the painter (`PaintError::DuplicateName`, 1b); a hidden or `display: none`
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

## In an ElementsApp

`ElementsApp::mount(name, renderer)` mounts a renderer before the
first frame; the painter is made on that frame and the app hands it
the renderers then. A taken name is refused at `mount`
(`MountError::AlreadyMounted`), not on the frame. `ElementsApp::unmount`
hands a renderer back. After each frame the app returns
`Next::Continue` while the document changed, a surface was
invalidated, or `ScenePainter::wants_frame()`; otherwise `Next::Wait`,
so an idle app draws nothing.

`mount` returns a `SurfaceHandle`, cheap to clone and single-threaded
as the document is. Its `invalidate()` marks the surface for a render
on the next frame and answers in three states. While `run` owns the
app it wakes the loop with the platform's `Waker`, which the app keeps
from its first event or frame (`Invalidated::Woke`). Before then the
loop has not started, and its first frame renders the surface with no
wake (`Invalidated::AtFirstFrame`). It refuses a surface since
unmounted (`InvalidateError::NotMounted`) and a loop that has ended:
a frame failed, the app was dropped, or the wake was refused
(`InvalidateError::LoopClosed`).

## Supported composition (first release)

| Case | Behaviour |
|---|---|
| Ancestor `overflow: hidden` or `clip`, rectangular | Clipped: the composite draws under the run's scissor |
| UI painted later (siblings, children, positioned elements) | Drawn over the surface |
| Panel resize, device scale change | The backing texture is reallocated at the new device-pixel size |
| Two surfaces, one name | Refused |
| Backing over the device texture limit | Refused |
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

## Renderer contract

`vapors_app::SurfaceRenderer`, mounted with `ScenePainter::mount(name,
renderer)`; `unmount` hands it back and dropping it is teardown.

- Device and queue are the application's, shared with the painter.
- `mount(device)` once, on the first frame that paints the surface:
  persistent resources.
- `resize(device, size)` when the backing is allocated: first paint, a
  change of device size or scale, and the first paint after the surface
  was inactive. `SurfaceSize` gives device pixels and the scale;
  `logical()` the CSS size.
- `render(frame)` records into a vapors-owned `CommandEncoder`, which the
  painter submits before compositing. It runs after a resize, after
  `ScenePainter::invalidate(name)`, and every frame while `animating()`.
  Other frames composite the last render: an idle surface renders
  nothing.
- `ScenePainter::wants_frame()` is true while an active surface is
  invalidated or animating, and for a renderer mounted since the last
  frame; the app returns `Next::Continue` then.
- Backing: one texture per surface, `SURFACE_FORMAT` (`Rgba8Unorm`)
  holding sRGB-encoded values with straight alpha, the `img`
  convention; composited with alpha blending, unscaled (the backing is
  the content box's device size), under the run's scissor. No tone
  mapping or colour management is applied by the surface.
- A surface not painted in a frame (hidden, zero-sized, `display:
  none`, wholly clipped, or removed) is inactive: its backing is
  released and it is not rendered. A refused frame also releases the
  backings it did not reach. The renderer is not told: it keeps its own
  resources until it is unmounted. A `deactivate` hook, called when the
  backing is released, would let it free them; there is none yet.

Refused, failing the frame (`PaintError`), each naming the surface:
`Unmounted` (no renderer), `DuplicateName` (two surfaces with one name
in a frame), `TooLarge` (backing over the device's
`max_texture_dimension_2d`), `Renderer` (an error from `mount`,
`resize` or `render`, or a GPU validation error made in one, caught by
an error scope around each call, `render`'s around its submit too; the
frame is never shown with stale or blank content). After a caught GPU
error every frame of that surface is refused until it is remounted.
Native wgpu resolves the scope in the frame; on the web it resolves
later, and a frame of the surface within 10 of its frames is refused
instead; a scope unresolved after 10 frames refuses the surface as
such. An active surface with an unresolved scope wants a frame. A scope
still unresolved at `unmount` is dropped unreported: the renderer is
gone and nothing it drew is shown again. A wholly clipped surface is refused for `Unmounted`,
`DuplicateName` and `TooLarge` as one in view is, though it is not
rendered, so scrolling a surface into view cannot be the first time it
fails. `ElementsApp` reports a failed frame and stops. `mount`,
`unmount` and `invalidate` refuse a taken or unknown name
(`MountError`).

GPU loss is reported by the platform frame loop
(`crates/platform/src/device_lost.rs`), which stops the app; surfaces
rely on it and attempt no recovery.
