<script>
use vapors_scene3d_components::{Group, GroupProps, Scene, SceneProps};

#[prop] let scene: Option<std::rc::Rc<std::cell::RefCell<vapors_scene3d::Scene>>> = None;
#[prop] let surface: Option<vapors_app::SurfaceHandle> = None;
#[prop] let outer: Signal<vapors_scene3d::Transform> = Signal::new(vapors_scene3d::Transform::IDENTITY);
#[prop] let inner: Signal<vapors_scene3d::Transform> = Signal::new(vapors_scene3d::Transform::IDENTITY);
#[prop] let shown: Signal<bool> = Signal::new(false);
</script>

<style>
.view { display: block; width: 64px; height: 64px; }
</style>

<gpu-surface name="view" class="view"></gpu-surface>
<Scene scene={scene} surface={surface}>
  <Group transform={outer}>
    <Group transform={inner} />
    {if shown.get()}
      <Group />
    {/if}
  </Group>
</Scene>
