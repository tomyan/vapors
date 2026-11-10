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
| 1b2 | `ElementsApp` mounting, `SurfaceHandle`, `Next` from `wants_frame`; the gradient example | #141, #142 |
| 1c | animation requests (PR 1, #147); the racer's renderer mounted through the surface (PR 2, #150); the racer's per-frame counts (PR 3, #161); two panels (PR 4, #163); surface-local input (PR 5, #164); resize, device scale and unmount on the racer (PR 6, this branch). Plan: `docs/plans/3d-1c.md` | done |
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

### An app that wraps ElementsApp

An app that draws under the document and owns state the document does
not (the racer: its race, its keys) wraps an `ElementsApp` and is the
platform `App` itself. Each frame it does its own work, then calls the
inner `frame`. For each event it calls `ElementsApp::give(&event,
&platform)` in place of `App::event`: the document gets the event by
`DocumentInput::give`'s rules (`document_input_route.rs`), and the
answer says who it was for. `Route::Document`: the document had it alone
(a press on its elements, a key its focused control used, an edit or
paste from the platform's text input that the focused field took).
`Route::Both`: the document had it and the app acts on it too (a key the
focused element left, Tab with nothing focused, which focuses the first
control, a window focus change). `Route::Host`: the app's alone; the
document did not get it (a press where no element takes the pointer,
another key with nothing focused). `give` refuses with a `GiveError`: a
pointer event the document cannot place (`GiveError::Unplaced`,
`NotUpdated` before the first frame), which the document does not get;
an edit from the text input the document refused (`EditRefused`); a paste
from it with no text field focused (`PasteWithoutField`). The racer
stops on the last two. `App::event` gives the document every event, as
before, and reports refused text input and goes on: an `ElementsApp`
alone has the window to itself. One `ElementsApp` takes events one way;
a debug build refuses `give` and `App::event` on the same instance.

When the inner `frame` returns `Next::Exit`, the `ElementsApp` failed
and closed its surfaces: the wrapping app returns `Exit` at once, and
whoever ran it reads the `Failure` cloned before `run` (the racer's
`run` returns it as `Err`).

### Animation

The app, not the renderer, decides when a surface animates (touche
decision e3, `2026-11-10-incremental-ui-contract-answers.md`).
`SurfaceHandle::request_animation()` renders the surface on every frame
that paints it, from the next, until `stop_animation()`; it wakes the
loop and answers as `invalidate` does. Each frame passes the app's
animating surfaces to the painter (`ScenePainter::set_animating`).
Requests from several surfaces coalesce: one frame renders each
animating surface once, and the app returns `Next::Continue` while any
active surface animates. When none does and nothing else wants a frame
the app returns `Next::Wait`. `stop_animation()` refuses an unmounted
surface (`NotMounted`) and does nothing once the loop has ended.

A request on a surface no frame paints (hidden, zero-sized, `display:
none`, clipped out) wakes one frame, which renders nothing; the app then
asks for no more. The request is kept, and the surface animates again
from the first frame that paints it. Unmounting forgets it.

At the end of each frame, before it decides whether to ask for the next,
the app passes the painter the current animating set
(`AppSurfaces::sync_animating`; a debug assertion checks that the painter
agrees for each painted surface). A `stop_animation` made during the
frame, even after its paint (a frame-times hook), therefore ends that
frame's request for the next. One made after the frame returned (in
`drawn`, or an event) cannot recall the frame already asked for: that
frame renders nothing, then the app waits. `surface_gradient --check`
stops from `drawn` and its idle count starts after that frame.

`ElementsApp::surface_states()` and `last_decision()` report each surface's
flags on the painter and why the last frame asked for another; a failing
check prints them (`surface_gradient --check`).

`SurfaceHandle` is `Rc` and not `Send`, like the document. A worker
thread sends its request to the app's thread (a channel) and wakes the
loop with the platform's `Waker`, which is `Send`; the app's thread
calls `request_animation` on the frame or event that follows.

`crates/app/examples/surface_gradient` draws a WGSL gradient in a
surface beside a panel of text, moving it a step at each render; run
without flags, the app requests animation before the loop starts. With
`--check` (the CI window job) it counts frames and renders over an idle
second, which must be none; then the running app invalidates the
surface from `drawn`, which must draw 2 frames (the wake that reached
the app, the invalidate's own) and render once; requests animation,
which must draw at least 10 frames in a second and render in each but
the wake's; stops it, after which an idle second must draw nothing;
after the loop ends an invalidate must be refused as `LoopClosed`.

### Two panels

`crates/app/examples/surface_panels` puts two surfaces in two panels
side by side, each with its own gradient renderer and handle: the left
animates throughout, the right is started and stopped every 2 s
(`cargo run -p vapors-app --example surface_panels`). It is not run in
CI and has no `--check`: its 2 s start/stop timing is checked by the
unit tests in `examples/surface_panels/flip.rs`, and
`tests/gpu_app_two_panels.rs` and `gpu_app_two_panels_resize.rs` hold
the GPU checks: while both
animate each renders once per frame; with one stopped it renders
nothing and the other renders every frame; a renderer filling a backing
larger than its `overflow: hidden` panel leaves the pixels just outside
the panel at the page background, and UI painted after both panels
stays over them; a panel resized by layout resizes its surface once and
renders at the new size; `FrameCounts::surfaces` counts one
composition draw per surface, a stopped one included.

### Resize, scale and unmount on the racer

The racer's `Track` renderer allocates its depth texture in `resize`, at
the backing's device size. Tests: `crates/racer/src/gpu_track_resize_tests.rs`
and `gpu_track_unmount_tests.rs`.

- A window resized while racing calls `resize` once, on the first frame
  at the new size; that frame and every later one render at the new
  size, and the depth texture is the new size. The HUD lays out for the
  new size on the same frame (below 760 CSS pixels wide, its compact
  layout).
- A device scale change from 1x to 2x and back calls `resize` once per
  change. At 2x the backing is twice the device size and `logical()`
  is the same CSS size; `FrameCounts::surfaces` counts one composition
  draw at each scale. With the race's clock held, a CSS point on the
  track has the same colour at 1x as at 2x.
- Unmounting while animating (on a test page holding only the track's
  `gpu-surface`, as the racer's HUD always has one): the surface
  element is removed and `ElementsApp::unmount` hands the renderer
  back; dropping it drops the `Track` and the GPU resources it owns
  (pipelines, buffers, depth texture). Its handle then refuses
  `request_animation`, `invalidate` and `stop_animation` with
  `InvalidateError::NotMounted`, naming the surface. The frame that
  shows the removal asks for one more, as any document change does;
  the frame after it returns `Next::Wait`. A new `Track` mounted on a
  new surface with the same name renders, and does not inherit the old
  one's animation request.
- The order of removing the element and unmounting does not matter for
  release: unmounting first and then removing the element drops the
  renderer and the page then waits. Unmounting and leaving the element
  in the document fails the next frame, naming `ScenePainter::mount` as
  the fix, and draws nothing stale. `gpu_track_unmount_order_tests.rs`.
- Resources are released by Rust ownership: dropping the renderer
  `unmount` returned drops the `Track` and what it owns, which
  `TrackLog::dropped` shows in each order. There is no separate release
  call.

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

A surface is an element: pointer events reach it by the document's hit
testing, dispatch and capture (`events_capture.rs`), not through the
renderer. The app listens on the `gpu-surface` element, or on an
ancestor, as on any element; an app that wraps `ElementsApp` gives each
event with `ElementsApp::give`, and a press on a surface is
`Route::Document`. Tests: `crates/app/tests/elements_app_surface_input.rs`
`elements_app_surface_input_state.rs` and `elements_app_surface_capture.rs`, on two panels each holding a
surface, with a button positioned over the left one.

- `Event::offset` is from the surface's content box top-left (its
  padding box, as padding is refused) in CSS pixels: the platform's
  physical position divided by the scale factor, less where the box is
  painted, so a scroll container's offset is included. Multiply by
  `SurfaceSize::scale` for backing pixels. vapors refuses `transform`,
  so no ancestor transform applies.
- UI painted over a surface is hit first (tested with a positioned
  button; a later non-positioned sibling covering it by a negative margin
  is not tested): a press on it goes to it, and the surface gets no event
  for that press.
- Capture is the web's. A mouse press on a surface goes on to the
  surface past its edge only when a `pointerdown` listener calls
  `set_pointer_capture`; then every move and the release go to it,
  with offsets negative or beyond its size, and the `click` goes to it
  as well. A touch is captured by what it went down on without asking.
  The capture ends at the release; later moves go to whatever is under
  the pointer.
- A press that starts on UI over a surface stays with the UI when the
  UI captures it. When it does not, as on the web, the surface under
  the pointer gets the moves and the release, though not the
  `pointerdown`; the `click` goes to the nearest common ancestor of
  the pressed and released elements (the body, for two panels), as in
  Chromium (recorded: `crates/elements-corpus/fixtures/cross_drag.events.json`).
- Hover: the surface gets `pointermove` (not pressed) while the
  pointer is over it and not over UI above it, and `:hover` applies
  (tested: `elements_app_surface_input_state.rs`; the surface is not
  hovered while the pointer is over a HUD button above it). vapors sends
  no `pointerover`, `pointerenter`, `pointerout` or `pointerleave`, so
  nothing tells a surface that the pointer left it or the window; the
  app sees moves reach another element. Chromium does send all four, so
  this is a known gap, not the web's behaviour. The document refuses a
  listener for any of them (`a_listener_for_enter_leave_over_or_out_is_refused_so_vapors_sends_none`),
  so an app cannot wait for one.
- `pointer-events: none` on a surface lets presses through to what is
  under it (tested: the panel below takes the press). The racer's HUD
  (`crates/racer/ui/hud.wisp`) puts `pointer-events: none` on `html` and
  `body` and `auto` on buttons; its track is a `gpu-surface` child of
  the body, so it inherits `none`.
  `pointer_events_none_on_the_body_reaches_a_surface_but_not_a_button_with_auto`
  tests that CSS shape on the test page, not the racer itself; the racer
  has no test that a press on its track reaches nothing.
- Keyboard: a `gpu-surface` is not focusable unless it has a
  `tabindex`, when it takes focus from a press and from Tab, as any
  element does (tested: a press and Tab).
- Not the web's: a touch that drags past the slop ends with `pointerup`
  on the captured surface and no `click`; Chromium cancels the touch at
  the first pan (`pointercancel`). A tap, which the platform sends after
  a short touch, ends in a `click`.

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
  `ScenePainter::invalidate(name)`, and every frame while the surface
  is animating (`ScenePainter::set_animating(name, true)`, which
  `SurfaceHandle::request_animation` drives). Other frames composite the
  last render: an idle surface renders nothing. The renderer does not
  ask for frames.
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
later, and the first frame of the surface after the result arrives is
refused instead. An unresolved scope asks for no frame. On the web it
is raced against a page timer at its deadline, in a task on the page's
queue (`painter_surface_scope_watch.rs`, `vapors_platform::Timer`).
An error, or the deadline passing first, wakes the loop, with the waker
`ElementsApp` gives the painter (`ScenePainter::wake_with`, from
`drawn` once the loop runs), for one frame that refuses the surface. A
result with no error clears the timer and wakes nothing, so an idle app
draws nothing. The deadline is 1000 ms (`DEADLINE` in
`painter_surface_scope.rs`, set from `scripts/web-scope-latency.mjs`:
the largest of 6,500 web results took 37.6 ms). On the web only time
with the page visible counts: a timer period that began or ended with
the page hidden (`document.hidden`, read through
`vapors_platform::page_hidden`) is re-armed for the full 1000 ms, so a
scope is refused after 1000 to 2000 ms of visible time
(`painter_surface_scope_deadline.rs`). Each surface's scopes are also
settled at the end of every frame, painted or not; natively one past
its deadline refuses the surface with the same message, while on the
web the timer alone judges lateness, so the first frame after a hidden
tab returns does not refuse a scope whose deadline passed while hidden.
Natively wgpu resolves every scope when it is popped, so nothing waits.
When a surface is unmounted or remounted with a scope unresolved, its
watch holds the result only weakly: when the scope's result or the
timer next arrives, the watch ends, dropping the timer, recording
nothing and waking nothing, so no frame follows. A wholly clipped surface is refused for `Unmounted`,
`DuplicateName` and `TooLarge` as one in view is, though it is not
rendered, so scrolling a surface into view cannot be the first time it
fails. `ElementsApp` reports a failed frame and stops. `mount`,
`unmount`, `invalidate` and `set_animating` refuse a taken or unknown
name (`MountError`).

GPU loss is reported by the platform frame loop
(`crates/platform/src/device_lost.rs`), which stops the app; surfaces
rely on it and attempt no recovery.
