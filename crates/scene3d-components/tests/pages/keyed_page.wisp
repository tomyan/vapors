<script>
use vapors_scene3d_components::{Group, GroupProps, Mesh, MeshProps, Scene, SceneProps};

#[prop] let scene: Option<std::rc::Rc<std::cell::RefCell<vapors_scene3d::Scene>>> = None;
#[prop] let surface: Option<vapors_app::SurfaceHandle> = None;
#[prop] let square: Option<vapors_scene3d::GeometryHandle> = None;
#[prop] let braking: Signal<bool> = Signal::new(false);

fn wheel_at(x: f32) -> Signal<vapors_scene3d::Transform> {
    let place = vapors_scene3d::Vec3::new(x, 0.0, 0.0);
    Signal::new(vapors_scene3d::Transform::from_translation(place).unwrap())
}
</script>

<style>
:global(body) { color: #0000ff; }
.view { display: block; width: 64px; height: 64px; }
.body { color: #ff0000; }
.braking { color: #00ff00; }
</style>

<gpu-surface name="view" class="view"></gpu-surface>
<Scene scene={scene} surface={surface}>
  <Group class="craft">
    <Mesh geometry={square.clone()} class="body" class:braking={braking.get()} />
    {for x in [-0.5f32, 0.5] key=x.to_bits()}
      <Mesh geometry={square.clone()} transform={wheel_at(x.get())} />
    {/for}
  </Group>
</Scene>
