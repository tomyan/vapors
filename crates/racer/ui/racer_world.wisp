<script>
// The racer's world in the scene (one-scene 2.2): drawn by the
// SceneRenderer on the HUD's gpu-surface named "track". The parts'
// colours are `world_geometry.rs`'s `Part::colour`, rounded to 8 bits.
use crate::world_geometry::Part;
use crate::world::piece;
use vapors_scene3d_components::{Camera, CameraProps, Group, GroupProps, Mesh, MeshProps, Scene, SceneProps};

#[prop] let scene: Option<std::rc::Rc<std::cell::RefCell<vapors_scene3d::Scene>>> = None;
#[prop] let surface: Option<vapors_app::SurfaceHandle> = None;
#[prop] let eye: Option<vapors_scene3d::CameraHandle> = None;
#[prop] let lens: Option<Signal<vapors_scene3d::Projection>> = None;
#[prop] let look: Signal<vapors_scene3d::Transform> = Signal::new(vapors_scene3d::Transform::IDENTITY);
#[prop] let world: Signal<vapors_scene3d::Transform> = Signal::new(vapors_scene3d::Transform::IDENTITY);
#[prop] let car: Signal<vapors_scene3d::Transform> = Signal::new(vapors_scene3d::Transform::IDENTITY);
#[prop] let ground: Option<vapors_scene3d::GeometryHandle> = None;
#[prop] let ground_material: Option<vapors_scene3d::MaterialId> = None;
#[prop] let parts: Vec<(crate::world_geometry::Part, vapors_scene3d::GeometryHandle)> = Vec::new();
</script>

<style>
.building { color: #c2ba96; }
.roof { color: #4a6666; }
.trunk { color: #573d26; }
.foliage { color: #4a7a54; }
.wheels { color: #12171a; }
.body { color: #f5944a; }
.cabin { color: #305461; }
.car-roof { color: #f59e52; }
.stripes { color: #f2e0b8; }
</style>

<Scene scene={scene} surface={surface} camera={eye}>
  <Camera handle={eye} projection={lens} transform={look} />
  <Group transform={world}>
    <Mesh geometry={ground.clone()} material={ground_material} />
    <Mesh geometry={piece(&parts, Part::Building)} class="building" />
    <Mesh geometry={piece(&parts, Part::Roof)} class="roof" />
    <Mesh geometry={piece(&parts, Part::Trunk)} class="trunk" />
    <Mesh geometry={piece(&parts, Part::Foliage)} class="foliage" />
    <Group transform={car}>
      <Mesh geometry={piece(&parts, Part::Wheels)} class="wheels" />
      <Mesh geometry={piece(&parts, Part::Body)} class="body" />
      <Mesh geometry={piece(&parts, Part::Cabin)} class="cabin" />
      <Mesh geometry={piece(&parts, Part::CarRoof)} class="car-roof" />
      <Mesh geometry={piece(&parts, Part::Stripes)} class="stripes" />
    </Group>
  </Group>
</Scene>
