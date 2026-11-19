<script>
use vapors_scene3d_components::{Light, LightProps, Scene, SceneProps};

#[prop] let scene: Option<std::rc::Rc<std::cell::RefCell<vapors_scene3d::Scene>>> = None;
#[prop] let surface: Option<vapors_app::SurfaceHandle> = None;
#[prop] let sun: Option<vapors_scene3d::Light> = None;
#[prop] let lamp: Option<vapors_scene3d::Light> = None;
#[prop] let lamps: Signal<u32> = Signal::new(0);
#[prop] let place: Signal<vapors_scene3d::Transform> = Signal::new(vapors_scene3d::Transform::IDENTITY);
#[prop] let ambient: vapors_scene3d::Ambient = vapors_scene3d::Ambient::NONE;

fn spaced(i: u32) -> ReadSignal<vapors_scene3d::Transform> {
    let x = f32::from(u16::try_from(i).expect("a few lamps"));
    let at = vapors_scene3d::Vec3::new(x, 0.0, 0.0);
    Signal::new(vapors_scene3d::Transform::from_translation(at).unwrap()).read_only()
}
</script>

<style>
.view { display: block; width: 64px; height: 64px; }
</style>

<gpu-surface name="view" class="view"></gpu-surface>
<Scene scene={scene} surface={surface} ambient={ambient}>
  <Light kind={sun} transform={place.read_only()} />
  {for i in (0..lamps.get()).collect::<Vec<u32>>() key=i}
    <Light kind={lamp} transform={spaced(i.get())} />
  {/for}
</Scene>
