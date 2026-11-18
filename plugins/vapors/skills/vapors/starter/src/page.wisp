<script>
use std::cell::RefCell;
use std::rc::Rc;
use vapors_elements::{frame_delta, frame_time};
use vapors_scene3d::{CameraHandle, Projection, Scene as Store};
use vapors_scene3d_components::{Camera, CameraProps, Group, GroupProps, Mesh, MeshProps, Scene, SceneProps};
use crate::game::{ground, pieces_at, Ground, Kind, Piece, World, HEIGHT, WIDTH};
use crate::shapes::Shapes;
use crate::view::{at, behind, heading, lying};

#[prop] let scene: Option<Rc<RefCell<Store>>> = None;
#[prop] let surface: Option<vapors_app::SurfaceHandle> = None;
#[prop] let shapes: Option<Shapes> = None;
#[prop] let eye: Option<CameraHandle> = None;
#[prop] let lens: Option<Signal<Projection>> = None;

let cuboid = shapes.as_ref().map(|s| s.cuboid.clone());
let wedge = shapes.as_ref().map(|s| s.wedge.clone());
let octagon = shapes.as_ref().map(|s| s.octagon.clone());

let world: Signal<World> = Signal::new(World::new());
// the frame's time and the time since the last frame: reading either
// makes the page draw every display refresh
let time = frame_time();
let delta = frame_delta();
let pieces: Memo<Vec<Piece>> = Memo::new(move || pieces_at(time.get().as_secs_f32()));
Effect::new(move || {
    let dt = delta.get().as_secs_f32();
    let mut next = world.get_untracked();
    next.tick(&pieces_at(time.get_untracked().as_secs_f32()), dt);
    world.set_if_changed(next);
});
let key = move |event: &mut Event| {
    let mut next = world.get_untracked();
    if next.key(event.key()) {
        event.prevent_default();
        world.set(next);
    }
};
let rows: Vec<i32> = (0..HEIGHT).collect();
let middle = (WIDTH - 1) as f32 / 2.0;
</script>

<style>
:global(html), :global(body) { width: 100%; height: 100%; margin: 0; overflow: hidden; }
:global(body) { background-color: #9fc6d8; color: #ffffff; font-size: 18px; }
.view { position: absolute; left: 0; top: 0; right: 0; bottom: 0; }
.hud { position: absolute; left: 16px; top: 12px; right: 16px; display: flex; gap: 24px; }
.title { font-size: 22px; }
.over { position: absolute; left: 0; right: 0; top: 40%; text-align: center; font-size: 28px; }
.grass { color: #6aa84f; }
.road { color: #545b66; }
.water { color: #3d85c6; }
.car { color: #e06650; }
.truck { color: #f1c232; }
.cabin { color: #d9e2ea; }
.log { color: #8d5a34; }
.frog { color: #93e05a; }
</style>

<vapors:window onkeydown={key} />
<gpu-surface name="view" class="view"></gpu-surface>
<Scene scene={scene} surface={surface} camera={eye}>
  <Camera handle={eye} projection={lens} transform={Signal::new(behind())} />
  {for row in rows.clone()}
    <Mesh geometry={cuboid.clone()} class="grass"
      class:road={ground(row.get()) == Ground::Road} class:water={ground(row.get()) == Ground::Water}
      transform={Signal::new(at((middle, -0.15, row.get() as f32), 0.0, (WIDTH as f32 + 6.0, 0.3, 1.0)))} />
  {/for}
  {for piece in pieces.get()}
    {if piece.get().kind == Kind::Log}
      <Mesh geometry={octagon.clone()} class="log"
        transform={Signal::derived(move || { let p = piece.get(); lying((p.x, 0.1, p.row as f32), p.kind.length(), 0.7) })} />
    {else}
      <Group transform={Signal::derived(move || { let p = piece.get(); at((p.x, 0.0, p.row as f32), heading(p.speed), (1.0, 1.0, 1.0)) })}>
        <Mesh geometry={cuboid.clone()} class="car" class:truck={piece.get().kind == Kind::Truck}
          transform={Signal::new(at((0.0, 0.25, 0.0), 0.0, (0.75, 0.45, piece.get().kind.length())))} />
        <Mesh geometry={wedge.clone()} class="cabin"
          transform={Signal::new(at((0.0, 0.62, 0.0), 0.0, (0.6, 0.3, 0.6)))} />
      </Group>
    {/if}
  {/for}
  <Mesh geometry={octagon.clone()} class="frog"
    transform={Signal::derived(move || { let (x, row) = world.get().frog; at((x, 0.2, row as f32), 0.0, (0.6, 0.4, 0.6)) })} />
</Scene>
<div class="hud">
  <div class="title">Dodge</div>
  <div>Score {world.get().score}</div>
  <div>Lives {world.get().lives}</div>
</div>
{if world.get().lives == 0}
  <div class="over">Game over. Press any key.</div>
{/if}
