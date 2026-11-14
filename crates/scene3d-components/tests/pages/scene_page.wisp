<script>
use vapors_scene3d_components::{Camera, CameraProps, Group, GroupProps, Mesh, MeshProps, Scene, SceneProps};

#[prop] let scene: Option<std::rc::Rc<std::cell::RefCell<vapors_scene3d::Scene>>> = None;
#[prop] let surface: Option<vapors_app::SurfaceHandle> = None;
#[prop] let square: Option<vapors_scene3d::GeometryHandle> = None;
#[prop] let eye: Option<vapors_scene3d::CameraHandle> = None;
#[prop] let lens: Option<vapors_scene3d::Projection> = None;
#[prop] let back: Signal<vapors_scene3d::Transform> = Signal::new(vapors_scene3d::Transform::IDENTITY);
#[prop] let left: Signal<vapors_scene3d::Transform> = Signal::new(vapors_scene3d::Transform::IDENTITY);
#[prop] let blue: Signal<bool> = Signal::new(false);
#[prop] let gone: Signal<bool> = Signal::new(false);
#[prop] let right: Signal<vapors_scene3d::Transform> = Signal::new(vapors_scene3d::Transform::IDENTITY);
#[prop] let shown: Signal<bool> = Signal::new(false);
</script>

<style>
:global(html) { --tint: #ff0000; }
:global(html.night) { --tint: #ffff00; }
:global(body) { color: var(--tint); }
.view { display: block; width: 64px; height: 64px; }
.left { color: var(--tint); }
.blue { color: #0000ff; }
.gone { visibility: hidden; }
</style>

<gpu-surface name="view" class="view"></gpu-surface>
<Scene scene={scene} surface={surface} camera={eye}>
  <Camera handle={eye} projection={lens} transform={back} />
  <Group>
    <Mesh geometry={square.clone()} class="left" class:blue={blue.get()} class:gone={gone.get()} transform={left} />
    {if shown.get()}
      <Mesh geometry={square.clone()} transform={right} />
    {/if}
  </Group>
</Scene>
