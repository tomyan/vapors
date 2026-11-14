<script>
use vapors_scene3d_components::{Group, GroupProps, Mesh, MeshProps, Scene, SceneProps};

#[prop] let scene: Option<std::rc::Rc<std::cell::RefCell<vapors_scene3d::Scene>>> = None;
#[prop] let surface: Option<vapors_app::SurfaceHandle> = None;
#[prop] let square: Option<vapors_scene3d::GeometryId> = None;
#[prop] let rows: Signal<Vec<i32>> = Signal::new(Vec::new());

/// The row's place, following its number: a row reused for another
/// number would move its node.
fn following(x: Signal<i32>) -> Signal<vapors_scene3d::Transform> {
    let place = Signal::new(at(x.get_untracked()));
    vapors_signals::Effect::new(move || place.set(at(x.get())));
    place
}

fn at(x: i32) -> vapors_scene3d::Transform {
    #[allow(clippy::cast_precision_loss)] // a small row number
    let place = vapors_scene3d::Vec3::new(x as f32, 0.0, 0.0);
    vapors_scene3d::Transform::from_translation(place).unwrap()
}
</script>

<gpu-surface name="view"></gpu-surface>
<Scene scene={scene} surface={surface}>
  <Group>
    {for x in rows.get() key=x}
      <Mesh geometry={square} transform={following(x)} />
    {/for}
  </Group>
</Scene>
