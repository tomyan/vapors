<script>
use std::cell::RefCell;
use std::rc::Rc;
use vapors_elements::frame_time;
use vapors_scene3d::{CameraHandle, GeometryHandle, Projection, Scene as Store};
use vapors_scene3d_components::{Camera, CameraProps, Mesh, MeshProps, Scene, SceneProps};
use crate::game::{World, cars_at, HEIGHT, WIDTH};
use crate::view::{overhead, place};

#[prop] let scene: Option<Rc<RefCell<Store>>> = None;
#[prop] let surface: Option<vapors_app::SurfaceHandle> = None;
#[prop] let square: Option<GeometryHandle> = None;
#[prop] let eye: Option<CameraHandle> = None;
#[prop] let lens: Option<Signal<Projection>> = None;

let world: Signal<World> = Signal::new(World::new());
// the time of the current frame; reading it draws every display refresh
let time = frame_time();
let cars: Memo<Vec<(f32, i32)>> = Memo::new(move || cars_at(time.get().as_secs_f32()));
Effect::new(move || {
    let cars = cars.get();
    world.update(|world| world.run_over(&cars));
});
let key = move |event: &mut Event| world.update(|world| world.key(event.key()));
let rows: Vec<i32> = (0..HEIGHT).collect();
let middle = (WIDTH - 1) as f32 / 2.0;
</script>

<style>
:global(html), :global(body) { width: 100%; height: 100%; margin: 0; overflow: hidden; }
:global(body) { background-color: #101418; color: #f3f1e8; font-size: 18px; }
.view { position: absolute; left: 0; top: 0; right: 0; bottom: 0; }
.hud { position: absolute; left: 16px; top: 12px; }
.road { color: #3c4248; }
/* after .road: an element with both classes takes the later rule */
.grass { color: #2f6b3a; }
.car { color: #d5533a; }
.frog { color: #8bd450; }
</style>

<vapors:window onkeydown={key} />
<gpu-surface name="view" class="view"></gpu-surface>
<Scene scene={scene} surface={surface} camera={eye}>
  <Camera handle={eye} projection={lens} transform={Signal::new(overhead())} />
  {for row in rows.clone()}
    <Mesh geometry={square.clone()} class="road" class:grass={row.get() == 0 || row.get() == HEIGHT - 1}
      transform={Signal::new(place(middle, row.get() as f32, (WIDTH as f32, 0.96), 0.0))} />
  {/for}
  {for car in cars.get()}
    <Mesh geometry={square.clone()} class="car"
      transform={Signal::derived(move || { let (x, y) = car.get(); place(x, y as f32, (1.4, 0.7), 1.0) })} />
  {/for}
  <Mesh geometry={square.clone()} class="frog"
    transform={Signal::derived(move || { let (x, y) = world.get().frog; place(x as f32, y as f32, (0.7, 0.7), 2.0) })} />
</Scene>
<div class="hud">Arrow keys move. Crossings {world.get().crossings} · Hits {world.get().hits}</div>
