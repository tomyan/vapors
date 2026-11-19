# The Rust side of a vapors page

What an app's own Rust touches: starting a page in the browser, signals,
the frame loop, and the 3D scene. This page covers the
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
let options = WindowOptions { app_name: "hello".into(), title: "hello".into(), width: 640, height: 480 };
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
  `.get_untracked()`, `.with(|v| ..)`; `Signal::derived(move || ..)` returns
  a read-only `ReadSignal<T>`, as does `signal.read_only()` (reads only).
- `Memo::new(move || ..)`, `Effect::new(move || ..)`.
- `batch(|| ..)` runs effects once after several writes;
  `untrack(|| ..)` reads without subscribing.
- A component's signals and memos are public fields of its struct
  (`page.world.set(..)`); props are not.

## Per-frame motion

`ElementsApp` draws only when something changed: an idle page draws
nothing. In the browser a redraw runs on `requestAnimationFrame`, so at
most once per display refresh.

`vapors_elements::frame_time()` and `frame_delta()` return a
`ReadSignal<Duration>`: the time since the first frame, and the time since
the previous frame. While anything reads either, the page draws every
display refresh; when the last reader unmounts, it idles again. A
script reads them directly ([wisp.md](https://github.com/tomyan/vapors/blob/main/plugins/vapors/skills/vapors/reference/wisp.md#the-frame-clock)).
The starter's turning box, in `page.wisp`:

```rust
// the time since the first frame; reading it draws every display refresh
let time = frame_time();
```

```html
<Mesh geometry={cuboid.clone()} class="box"
  transform={Signal::derived(move || turned(time.get().as_secs_f32() * 0.6))} />
```

## The viewport

`vapors_elements::viewport_size()` (`ReadSignal<Size>`, CSS px) and
`orientation()` (`ReadSignal<Orientation>`: `Portrait` when taller than
wide, a square is `Landscape`) are app-wide, set only when the size
changes, and never draw a frame themselves. Use them for a camera's field
of view or where touch controls go. In CSS the same viewport answers
`@media (orientation: portrait)`, `(min-aspect-ratio: 16/9)`,
`(pointer: none | coarse | fine)` and `(hover: none | hover)`; the last two
come from the browser's `matchMedia` on the web, `fine` and `hover` natively.

To apply rules once a frame, read `frame_delta()` in an
`Effect::new(move || ..)`, compute the new state from the current one
(`get_untracked()`), and write it with `set_if_changed`, so the page
redraws only when something changed.

Read wall-clock time through `vapors_platform::Instant`, never
`std::time::Instant`, which panics in the browser.

### Simple games

A game uses the same three pieces: the frame clock to move things each
frame, `<vapors:window onkeydown={..} onkeyup={..} />` to read keys
whatever has focus (the canvas holds focus from the start), and the 3D
scene for anything that moves, since an element's position cannot
follow a signal. Keep the game's state and rules in a plain Rust module
with unit tests, and let the `.wisp` component read it through signals.

## The 3D scene

An element's position cannot follow a signal (`style={..}` is refused),
so anything that moves is a mesh in a 3D scene, with HTML drawn over it. Reference:
[docs/scene3d.md](https://github.com/tomyan/vapors/blob/main/docs/scene3d.md).

### What the scene has, and what it does not

- Meshes of triangles, each corner with a shade from 0 to 1, baked in
  when the geometry is made. The mesh's CSS `color` times the shade is
  what is drawn. That is all the shading there is.
- Depth testing, perspective and orthographic cameras, groups whose
  transforms nest.
- Not yet: lights, shadows, textures, transparency, materials beyond a
  colour, and loading model files. Do not imitate them (no fake light
  pools, glows, blob shadows or painted-on textures); build with flat
  colours and baked shades.

### Low-poly models in code

The starter's `src/shapes.rs` builds three solids, each face with its
own corners and one shade, so the facets read without lights: faces
pointing up are lightest, faces towards the camera lighter than those
facing away, faces pointing down darkest.

| Function | Shape, before its transform |
|---|---|
| `cuboid(meshes)` | a cube 1 wide centred on the origin; scale it into any box |
| `wedge(meshes)` | 1 wide, deep and tall: full height at the back (-Z), sloping to the front edge (+Z) |
| `cylinder(meshes, sides)` | `sides` flat sides around the Y axis, 1 tall and 1 across; 6 to 10 sides keeps it low-poly |

Make each once in `main.rs` (`Shapes::new`), pass the handles to the
page as a prop, and place, turn and scale copies with transforms. Build
a model from several meshes inside a `<Group>`, which moves them
together, such as a box with a wedge on top. Keep the
style smooth low-poly: a few chamfered or sloped shapes read better
than stacked cubes. For a new shape, add a function beside these that
lists its faces as convex polygons, counter-clockwise seen from
outside, and passes them to `solid`.

### In Rust

```rust
let scene = Rc::new(RefCell::new(Scene::new()));          // vapors-scene3d
let meshes = Rc::new(RefCell::new(Meshes::new()));
let renderer = SceneRenderer::new(Rc::clone(&scene), Rc::clone(&meshes)); // vapors-scene3d-gpu
let surface = app.mount("view", Box::new(renderer)).map_err(|e| e.to_string())?;
let shapes = shapes::Shapes::new(&mut meshes.borrow_mut()).map_err(|e| e.to_string())?;
```

Pass `scene`, `surface`, the shapes, a `CameraHandle::new()` and a
`ReadSignal<Projection>` to the page as props. `app.mount("view", ..)` draws
on the page's `<gpu-surface name="view">`.

- `Meshes::geometry(positions, indices, shades)` takes `Vec<Vec3>`
  corners, triangles as `u32` indices and one shade per corner, and
  returns a `GeometryHandle`; clone it for each mesh.
- `Projection::perspective(fov_radians, near, far)`, or
  `Projection::orthographic(height, near, far)` for a flat view.
- `Transform::new(translation, rotation, scale)` and
  `Transform::from_translation(v)` (`Vec3`, `Quat` from
  `vapors-scene3d`) return a `Result`: a non-finite value is refused.
- Conventions: right-handed, +X right, +Y up, +Z towards the viewer; a
  camera looks down its own -Z. The starter's camera sits in front of
  the box and above it, turned about X by -0.42 radians to look down
  at it.

### In the template

From the starter's `page.wisp`; `camera` and `turned` are its own
helpers in `view.rs`.

```html
<gpu-surface name="view" class="view"></gpu-surface>
<Scene scene={scene} surface={surface} camera={eye}>
  <Camera handle={eye} projection={lens} transform={Signal::new(camera()).read_only()} />
  <Mesh geometry={cuboid.clone()} class="box"
    transform={Signal::derived(move || turned(time.get().as_secs_f32() * 0.6))} />
</Scene>
```

Several meshes inside `<Group transform={..}>` move together, and a
`{for}` of meshes or groups adds and removes them with a list.

| Tag | Props |
|---|---|
| `<Scene>` | `scene`, `surface`, `camera` (all `Option`, all required) |
| `<Camera>` | `handle`, `projection: Option<ReadSignal<Projection>>`, `transform: ReadSignal<Transform>` |
| `<Group>` | `transform: ReadSignal<Transform>`, children |
| `<Mesh>` | `geometry: Option<GeometryHandle>`, `transform: ReadSignal<Transform>` |

- Import them in the script:
  `use vapors_scene3d_components::{Camera, CameraProps, Group, GroupProps, Mesh, MeshProps, Scene, SceneProps};`.
- A mesh's colour is its CSS `color`, set through `class` and
  `class:name={..}` on the tag, from the component's `<style>`. These
  rules are not checked at build time (#512).
- A transform that changes is a `Signal::derived(move || ..)`; each
  change redraws only that node.
- The props of these tags are checked by rustc, not by `wisp-check`,
  so a wrong prop name shows up as a rustc error in the generated file.
- Put HTML that goes over the scene after the `<Scene>`, positioned
  absolutely over the `gpu-surface`.
