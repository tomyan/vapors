//! Times `pick` against mesh count:
//! `cargo run --release -p vapors-scene3d --example pick_cost`.
//! Each scene is a column of unit cubes (12 triangles each) stacked down
//! -Z in front of an orthographic camera. Two points are picked: one
//! beside the column, where the ray enters no mesh's bounds (bounds
//! tests only), and one through its centre, where the ray enters every
//! mesh's bounds and tests all of its triangles (the worst case). Prints
//! the median and the 95th percentile of 200 picks after 20 warm-up
//! ones, on a 100 x 100 surface. CPU only.

use std::time::{Duration, Instant};

use vapors_scene3d::{
    CameraHandle, Colour, GeometryHandle, Meshes, NodeId, NodeKind, PickAt, Projection, Scene,
    Transform, Vec3, pick,
};

const COUNTS: [usize; 4] = [10, 100, 1_000, 10_000];
const BESIDE: [f32; 2] = [5.0, 50.0];
const THROUGH: [f32; 2] = [50.0, 50.0];

fn main() {
    for count in COUNTS {
        let (mut scene, meshes, nearest) = column(count);
        let beside = time(&mut scene, &meshes, BESIDE, None);
        let through = time(&mut scene, &meshes, THROUGH, Some(nearest));
        println!(
            "pick_cost: {count} meshes: bounds only median {:?} p95 {:?}; every triangle median {:?} p95 {:?}",
            beside.0, beside.1, through.0, through.1
        );
    }
}

/// The median and 95th percentile of 200 picks at `point`, failing
/// loudly when the pick refuses or picks other than `expect`.
fn time(
    scene: &mut Scene,
    meshes: &Meshes,
    point: [f32; 2],
    expect: Option<NodeId>,
) -> (Duration, Duration) {
    let at = PickAt {
        point,
        size: [100.0, 100.0],
    };
    let mut times = Vec::new();
    for round in 0..220 {
        let started = Instant::now();
        let picked = pick(scene, meshes, at, |_| false).unwrap_or_else(|error| fail(&error));
        if round >= 20 {
            times.push(started.elapsed());
        }
        if picked != expect {
            fail(&format!(
                "the pick at {point:?} returned {picked:?}, not {expect:?}"
            ));
        }
    }
    times.sort();
    (times[times.len() / 2], times[(times.len() - 1) * 95 / 100])
}

fn fail(why: &dyn std::fmt::Display) -> ! {
    eprintln!(
        "FAIL pick_cost: {why}; reproduce: cargo run --release -p vapors-scene3d --example pick_cost"
    );
    std::process::exit(1);
}

/// `count` unit cubes centred on the Z axis at z = 0, -2, -4, ..., a
/// camera at z = 5 whose far plane lies past the last one, and the
/// nearest cube.
fn column(count: usize) -> (Scene, Meshes, NodeId) {
    let mut scene = Scene::new();
    let mut meshes = Meshes::new();
    let cube = cube(&mut meshes);
    #[allow(
        clippy::cast_precision_loss,
        reason = "counts up to 10,000 are exact in f32"
    )]
    let depths: Vec<f32> = (0..count).map(|index| -2.0 * index as f32).collect();
    // the camera is 5 in front of z = 0; the far plane 5 past the last
    camera(&mut scene, 10.0 - depths[depths.len() - 1]);
    let root = scene.root();
    let cubes: Vec<NodeId> = (depths.iter())
        .map(|z| {
            let kind = NodeKind::Mesh {
                geometry: cube.clone(),
                colour: Colour::BLACK,
            };
            scene
                .add(root, kind, at(*z))
                .unwrap_or_else(|error| fail(&error))
        })
        .collect();
    (scene, meshes, cubes[0])
}

/// An orthographic camera 4 units high at z = 5, its far plane `far`
/// away, as the active camera.
fn camera(scene: &mut Scene, far: f32) {
    let projection = Projection::orthographic(4.0, 0.1, far).unwrap_or_else(|error| fail(&error));
    let (root, handle) = (scene.root(), CameraHandle::new());
    let kind = NodeKind::Camera { handle, projection };
    scene
        .add(root, kind, at(5.0))
        .unwrap_or_else(|error| fail(&error));
    scene.set_camera(Some(handle));
}

fn at(z: f32) -> Transform {
    Transform::from_translation(Vec3::new(0.0, 0.0, z)).unwrap_or_else(|error| fail(&error))
}

/// A unit cube centred on the origin: corner `i` has x, y, z from bits
/// 2, 1, 0 of `i`; two triangles per face.
fn cube(meshes: &mut Meshes) -> GeometryHandle {
    let side = |bit: usize, i: usize| if i >> bit & 1 == 1 { 0.5 } else { -0.5 };
    let positions = (0..8)
        .map(|i| Vec3::new(side(2, i), side(1, i), side(0, i)))
        .collect();
    let faces: [[u32; 4]; 6] = [
        [0, 1, 3, 2],
        [4, 6, 7, 5],
        [0, 4, 5, 1],
        [2, 3, 7, 6],
        [0, 2, 6, 4],
        [1, 5, 7, 3],
    ];
    let indices = faces
        .iter()
        .flat_map(|[a, b, c, d]| [*a, *b, *c, *a, *c, *d])
        .collect();
    meshes
        .geometry(positions, indices, vec![1.0; 8])
        .unwrap_or_else(|error| fail(&error))
}
