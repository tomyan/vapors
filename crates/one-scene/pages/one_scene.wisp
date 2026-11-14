<script>
use vapors_scene3d_components::{Camera, CameraProps, Group, GroupProps, Mesh, MeshProps, Scene, SceneProps};
use crate::parts::{at, looking, turning};
use crate::schedule::{gone, night};

#[prop] let scene: Option<std::rc::Rc<std::cell::RefCell<vapors_scene3d::Scene>>> = None;
#[prop] let surface: Option<vapors_app::SurfaceHandle> = None;
#[prop] let cube: Option<vapors_scene3d::GeometryId> = None;
#[prop] let ground: Option<vapors_scene3d::GeometryId> = None;
#[prop] let eye: Option<vapors_scene3d::CameraHandle> = None;
#[prop] let lens: Option<vapors_scene3d::Projection> = None;
#[prop] let time: Signal<std::time::Duration> = Signal::new(std::time::Duration::ZERO);
#[prop] let frames: Signal<u64> = Signal::new(0);

let lit1: Signal<bool> = Signal::new(false);
let lit2: Signal<bool> = Signal::new(false);
let lit3: Signal<bool> = Signal::new(false);
let toggle1 = move || lit1.update(|on| *on = !*on);
let toggle2 = move || lit2.update(|on| *on = !*on);
let toggle3 = move || lit3.update(|on| *on = !*on);

</script>

<style>
:global(html), :global(body) { width: 100%; height: 100%; margin: 0; overflow: hidden; }
:global(html) { --tint: #3b4a3f; }
:global(body) { color: #f3f1e8; font-size: 16px; font-family: sans-serif; background-color: #101418; }
.stage { position: absolute; left: 0; top: 0; right: 0; bottom: 0; }
.stage.night { --tint: #1d2340; }
.view { position: absolute; left: 0; top: 0; right: 0; bottom: 0; }
.hud { position: absolute; left: 16px; top: 16px; width: 264px; padding: 12px 16px; background-color: #182a2f; border-radius: 6px; }
.title { font-size: 20px; }
.frames { margin-top: 4px; font-size: 14px; }
.lights { margin-top: 8px; display: flex; column-gap: 8px; }
/* a fixed width: a flex row of buttons in an absolute panel wraps the
   first one's text (#346); remove once that is fixed */
button { width: 76px; background-color: #20383d; color: #f3f1e8; border: 1px solid #526a69; border-radius: 5px; padding: 8px 12px; font-size: 15px; }
.ground { color: var(--tint); }
.one { color: #c0392b; }
.two { color: #27ae60; }
.three { color: #2980b9; }
.lit { color: #f1c40f; }
.gone { visibility: hidden; }
</style>

<div class="stage" class:night={night(time.get())}>
  <gpu-surface name="view" class="view"></gpu-surface>
  <Scene scene={scene} surface={surface} camera={eye}>
    <Camera handle={eye} projection={lens} transform={looking()} />
    <Mesh geometry={ground} class="ground" transform={at(0.0, -0.5, 0.0)} />
    <Group transform={turning(time)}>
      <Mesh geometry={cube} class="one" class:lit={lit1.get()} transform={at(-2.0, 0.0, 0.0)} />
      <Mesh geometry={cube} class="two" class:lit={lit2.get()} transform={at(0.0, 0.0, 0.0)} />
      <Mesh geometry={cube} class="three" class:lit={lit3.get()} class:gone={gone(time.get())} transform={at(2.0, 0.0, 0.0)} />
    </Group>
  </Scene>
  <div class="hud">
    <div class="title">One scene</div>
    <div class="frames">Frame {frames}</div>
    <div class="lights">
      <button data-light="1" onclick={toggle1}>Light 1</button>
      <button data-light="2" onclick={toggle2}>Light 2</button>
      <button data-light="3" onclick={toggle3}>Light 3</button>
    </div>
  </div>
</div>
