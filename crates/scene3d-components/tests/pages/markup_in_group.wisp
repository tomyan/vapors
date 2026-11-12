<script>
use vapors_scene3d_components::{Group, GroupProps, Scene, SceneProps};

#[prop] let scene: Option<std::rc::Rc<std::cell::RefCell<vapors_scene3d::Scene>>> = None;
#[prop] let surface: Option<vapors_app::SurfaceHandle> = None;
</script>

<Scene scene={scene} surface={surface}>
  <Group>
    <p>Lap 3</p>
  </Group>
</Scene>
