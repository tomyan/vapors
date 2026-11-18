# The Rust side of a vapors page

What an app's own Rust touches: starting a page in the browser, signals,
the frame loop, and the 3D scene a game draws with. This page covers the
web build only. vapors also runs natively in a window (Linux, macOS,
Windows) with the same code; this skill does not cover that yet.

The starter's [main.rs](https://github.com/tomyan/vapors/blob/main/plugins/vapors/skills/vapors/starter/src/main.rs) is a complete example of
everything here.

## Starting a page

```rust
let document = Rc::new(RefCell::new(Document::new(FontSystem::new())));
let mut app = ElementsApp::new(Rc::clone(&document));
let body = document.borrow().body();
std::mem::forget(page::Page::mount(&document, body, page::PageProps { .. }));
vapors_platform::app_draws_open();   // the page has no Open button
vapors_platform::hide_status_line(); // #status would cover the page
let options = WindowOptions { app_name: "dodge".into(), title: "dodge".into(), width: 640, height: 480 };
vapors_platform::run(options, app).map_err(|e| e.to_string())
```

- `Document` (`vapors-elements`) is the element tree; `FontSystem`
  (`vapors-text`) gives it the bundled font.
- `Page::mount(&document, parent, props)` runs the component's script
  and builds its template. The returned struct owns the component's
  signals and effects: dropping it stops them, so keep it or
  `std::mem::forget` it for the life of the page.
- `ElementsApp` (`vapors-app`) lays the document out, draws it with
  wgpu, and turns browser input into DOM events.
- `run` attaches to `<canvas id="vapors">` and returns at once on the
  web; the browser owns the loop. `width` and `height` are ignored
  there: the canvas's CSS size is used.
- `report_error(&str)` writes to `#error` (to `#status` when the page
  has no `#error`) and the console; `report_status(&str)` replaces the
  text of `#status`. Send a failure in your start-up code to
  `report_error`, or the page stays at `starting`.

### The page contract

`index.html` needs `<canvas id="vapors">` sized by CSS
(`width: 100vw; height: 100vh; display: block; touch-action: none`) and
trunk's `<link data-trunk rel="rust" />`. `<p id="status">` and
`<p id="error" hidden>` are optional; without either, errors reach the
console only. The page needs WebGPU: Chrome or Edge 113+, Safari 26+,
Firefox 141+, over https or on localhost. There is no WebGL fallback.
Details: [docs/web.md](https://github.com/tomyan/vapors/blob/main/docs/web.md).

## Signals

`vapors-signals`, with Leptos's semantics. [wisp.md](https://github.com/tomyan/vapors/blob/main/plugins/vapors/skills/vapors/reference/wisp.md#script)
covers them as a script uses them. From Rust:

- `Signal::new(v)`, `.get()`, `.set(v)`, `.update(|v| ..)`,
  `.get_untracked()`, `.with(|v| ..)`; `Signal::derived(move || ..)`.
- `Memo::new(move || ..)`, `Effect::new(move || ..)`.
- `batch(|| ..)` runs effects once after several writes;
  `untrack(|| ..)` reads without subscribing.
- A component's signals and memos are public fields of its struct
  (`page.world.set(..)`); props are not.

## The frame loop

`ElementsApp` draws only when something changed: an idle page draws
nothing. In the browser a redraw runs on `requestAnimationFrame`, so at
most once per display refresh.

Pending (not yet in the pinned rev): `vapors_elements::frame_time()`
and `frame_delta()` return a `Signal<Duration>`: the current frame's
time, and the time since the last frame. While anything reads one, the
page draws every display refresh; when the last reader unmounts, it
idles again. A script uses them directly:

```rust
let time = frame_time();
let cars: Memo<Vec<(f32, i32)>> = Memo::new(move || cars_at(time.get().as_secs_f32()));
```

Underneath: an app's `frame` returns `Next::Continue` (draw again next
refresh), `Next::Wait` (draw on the next change) or
`Next::WaitUntil(instant)`. `Frame::now` is the frame's time, read once
as it starts (on the web, `performance.now()`).

Read time through `vapors_platform::Instant`, never
`std::time::Instant`, which panics in the browser.

## The 3D scene, for games

An element's position cannot follow a signal (`style={..}` is refused),
so anything that moves is a mesh in a 3D scene. Looking straight down
through an orthographic camera, the scene is a flat 2D picture with
HTML drawn over it. Reference:
[docs/scene3d.md](https://github.com/tomyan/vapors/blob/main/docs/scene3d.md).

### In Rust

```rust
let scene = Rc::new(RefCell::new(Scene::new()));          // vapors-scene3d
let meshes = Rc::new(RefCell::new(Meshes::new()));
let renderer = SceneRenderer::new(Rc::clone(&scene), Rc::clone(&meshes)); // vapors-scene3d-gpu
let surface = app.mount("view", Box::new(renderer)).map_err(|e| e.to_string())?;
let square = meshes.borrow_mut().geometry(positions, indices, shades)?;
```

Pass `scene`, `surface`, the geometry, a `CameraHandle::new()` and a
`Signal<Projection>` to the page as props. `app.mount("view", ..)` draws
on the page's `<gpu-surface name="view">`.

- `Meshes::geometry(positions, indices, shades)`: `Vec<Vec3>` corners,
  triangles as `u32` indices (counter-clockwise seen from the camera),
  and a shade from 0 to 1 per corner (the fraction of the mesh's colour
  drawn there). It returns a `GeometryHandle`; clone it for each mesh.
- `Projection::orthographic(height, near, far)`: `height` world units
  tall, its width following the canvas's aspect.
  `Projection::perspective(fov_radians, near, far)` for a 3D view.
- `Transform::new(translation, rotation, scale)` and
  `Transform::from_translation(v)` (`Vec3`, `Quat` from
  `vapors-scene3d`) return a `Result`: a non-finite value is refused.
- Conventions: right-handed, +X right, +Y up, +Z towards the viewer; a
  camera looks down its own -Z. A camera at `z = 10` with no rotation
  looks straight down at the XY plane.

### In the template

From the starter's `page.wisp`; `overhead` and `place` are its own
helpers in `view.rs`.

```html
<gpu-surface name="view" class="view"></gpu-surface>
<Scene scene={scene} surface={surface} camera={eye}>
  <Camera handle={eye} projection={lens} transform={Signal::new(overhead())} />
  {for car in cars.get()}
    <Mesh geometry={square.clone()} class="car"
      transform={Signal::derived(move || { let (x, y) = car.get(); place(x, y as f32, (1.4, 0.7), 1.0) })} />
  {/for}
</Scene>
```

| Tag | Props |
|---|---|
| `<Scene>` | `scene`, `surface`, `camera` (all `Option`, all required) |
| `<Camera>` | `handle`, `projection: Option<Signal<Projection>>`, `transform: Signal<Transform>` |
| `<Group>` | `transform: Signal<Transform>`, children |
| `<Mesh>` | `geometry: Option<GeometryHandle>`, `transform: Signal<Transform>` |

- Import them in the script:
  `use vapors_scene3d_components::{Camera, CameraProps, Group, GroupProps, Mesh, MeshProps, Scene, SceneProps};`.
- A mesh's colour is its CSS `color`, set through `class` and
  `class:name={..}` on the tag, from the component's `<style>`. These
  rules are not checked at build time (#512).
- A transform that changes is a `Signal::derived(move || ..)`; each
  change redraws only that node. A `{for}` of `<Mesh>` tags adds and
  removes meshes with the list.
- The props of these tags are checked by rustc, not by `wisp-check`,
  so a wrong prop name shows up as a rustc error in the generated file.
- Put HTML for the score or lives after the `<Scene>`, positioned
  absolutely over the `gpu-surface`.
