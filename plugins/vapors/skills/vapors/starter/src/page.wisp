<script>
use std::cell::RefCell;
use std::rc::Rc;
use vapors_elements::frame_time;
use vapors_scene3d::{CameraHandle, Projection, Scene as Store};
use vapors_scene3d_components::{Camera, CameraProps, Mesh, MeshProps, Scene, SceneProps};
use crate::shapes::Shapes;
use crate::view::{camera, turned};

#[prop] let scene: Option<Rc<RefCell<Store>>> = None;
#[prop] let surface: Option<vapors_app::SurfaceHandle> = None;
#[prop] let shapes: Option<Shapes> = None;
#[prop] let eye: Option<CameraHandle> = None;
#[prop] let lens: Option<Signal<Projection>> = None;

let cuboid = shapes.as_ref().map(|s| s.cuboid.clone());
// the time since the first frame; reading it draws every display refresh
let time = frame_time();
let last_key: Signal<String> = Signal::new("none yet".into());
let key = move |event: &mut Event| last_key.set(format!("{:?}", event.key()));
</script>

<style>
:global(html), :global(body) { width: 100%; height: 100%; margin: 0; overflow: hidden; }
:global(body) { background-color: #1b2228; color: #e8ecef; font-size: 18px; }
.view { position: absolute; left: 0; top: 0; right: 0; bottom: 0; }
.hud { position: absolute; left: 24px; top: 16px; }
h1 { margin: 0 0 8px; font-size: 28px; }
.box { color: #e07a4f; }
</style>

<vapors:window onkeydown={key} />
<gpu-surface name="view" class="view"></gpu-surface>
<Scene scene={scene} surface={surface} camera={eye}>
  <Camera handle={eye} projection={lens} transform={Signal::new(camera())} />
  <Mesh geometry={cuboid.clone()} class="box"
    transform={Signal::derived(move || turned(time.get().as_secs_f32() * 0.6))} />
</Scene>
<div class="hud">
  <h1>Hello from vapors</h1>
  <p>Last key: {last_key}</p>
</div>
