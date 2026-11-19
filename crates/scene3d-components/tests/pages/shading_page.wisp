<script>
use vapors_scene3d_components::{Mesh, MeshProps, Scene, SceneProps};

#[prop] let scene: Option<std::rc::Rc<std::cell::RefCell<vapors_scene3d::Scene>>> = None;
#[prop] let surface: Option<vapors_app::SurfaceHandle> = None;
#[prop] let square: Option<vapors_scene3d::GeometryHandle> = None;
#[prop] let shading: vapors_scene3d::Shading = vapors_scene3d::Shading::Unlit;
</script>

<gpu-surface name="view"></gpu-surface>
<Scene scene={scene} surface={surface}>
  <Mesh geometry={square.clone()} shading={shading} />
  <Mesh geometry={square.clone()} />
</Scene>
