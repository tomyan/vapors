//! GPU tier: the racer's ground as an application material (one-scene
//! R2) draws the ground's colours at a point on the road, the kerb, the
//! verge, the grass and the start stripe, each looked at straight down
//! from above on an odd-sized target, sampled at the centre (the
//! camera's ground point, the vignette 1) and at the middles of the
//! left, top and bottom edges (the vignette darker). The colours are
//! pinned as literals, taken from the Track renderer, which this test
//! also draws until R3a2 deletes it.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_gpu_harness::{Gpu, gpu};
use vapors_scene3d::{CameraHandle, GeometryHandle, Meshes, NodeId, NodeKind, Projection};
use vapors_scene3d::{Quat, Scene, Transform, Vec3};
use vapors_scene3d_gpu::SceneRenderer;

use crate::gpu::{Scene as TrackScene, Track, TrackLog};
use crate::model::{Camera, RADIUS, Race};

#[path = "gpu_ground_material_target.rs"]
mod target;
use target::{Samples, drawn, mount, sampled};

/// Device pixels a ground unit spans: the Track's `min(w, h) / 400 * zoom`.
const ZOOM: f32 = 6.0;

/// Points at distances from the bottom straight's centre line, `x` 60.5
/// (clear of the start stripe, the grid and the trees): road 15.5, kerb
/// 37.5, verge 43.5, grass 80.5; and a light square of the start stripe,
/// which only the bottom straight has. Each is on a half unit, so the
/// noise cells hold either side of it; the edge samples are about 33
/// units away (a unit is 0.975 pixels). With their colours at the
/// centre, left, top and bottom, as the Track drew them on 2026-11-17
/// (`by_track`).
const POINTS: [(&str, [f32; 2], [Pin; 4]); 5] = [
    ("road", [60.5, -RADIUS + 15.5], ROAD),
    ("kerb", [60.5, -RADIUS - 37.5], KERB),
    ("verge", [60.5, -RADIUS - 43.5], VERGE),
    ("grass", [60.5, -RADIUS - 80.5], GRASS),
    ("start", [3.5, -RADIUS + 10.5], START),
];

/// A pinned sample: its RGBA and how far a GPU's may be from it, a
/// channel.
type Pin = ([u8; 4], u8);

/// A sample where nothing is noisy (kerb, verge, start stripe): the
/// rounding of one channel.
const EXACT: u8 = 1;

/// A sample on grass or road, whose noise hashes `sin` of a large value,
/// which GPUs round differently: the noise spans 0.018 (5 levels), and
/// ANV (UHD 630) and llvmpipe drew these up to 4 apart on 2026-11-17.
const NOISY: u8 = 6;

const ROAD: [Pin; 4] = [
    ([47, 60, 63, 255], NOISY),
    ([46, 58, 61, 255], NOISY),
    ([44, 56, 59, 255], NOISY),
    ([55, 85, 75, 255], NOISY),
];
const KERB: [Pin; 4] = [
    ([229, 222, 191, 255], EXACT),
    ([190, 72, 57, 255], EXACT),
    ([62, 92, 82, 255], NOISY),
    ([43, 56, 58, 255], NOISY),
];
const VERGE: [Pin; 4] = [
    ([158, 161, 128, 255], EXACT),
    ([153, 156, 123, 255], EXACT),
    ([61, 91, 81, 255], NOISY),
    ([45, 57, 59, 255], NOISY),
];
const GRASS: [Pin; 4] = [
    ([64, 95, 85, 255], NOISY),
    ([64, 94, 84, 255], NOISY),
    ([61, 90, 81, 255], NOISY),
    ([65, 94, 83, 255], NOISY),
];
const START: [Pin; 4] = [
    ([240, 232, 199, 255], EXACT),
    ([44, 57, 59, 255], NOISY),
    ([40, 49, 52, 255], EXACT),
    ([153, 156, 123, 255], EXACT),
];

/// The racer's world in the scene (one-scene 2.2): Y mirrored, then
/// turned -90 degrees about X, so simulation (x, y, z) is the scene's
/// (x, z, y).
fn racers_world() -> Transform {
    let turned = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    Transform::new(Vec3::ZERO, turned, Vec3::new(1.0, -1.0, 1.0)).unwrap()
}

/// Above `[x, y]`, in the scene's root (outside the mirrored group,
/// whose mirror a camera inside would take into its view): looking down
/// -Y with up -Z, so simulation X runs right and Y down, as the Track
/// draws them.
fn above([x, y]: [f32; 2]) -> Transform {
    let turned = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    Transform::new(Vec3::new(x, 500.0, y), turned, Vec3::ONE).unwrap()
}

/// The Track renderer's ground at each point: looked at from straight
/// above (tilt 1), the car parked far out of view; one Track, mounted
/// once, its camera moved between points.
fn by_track(gpu: &Gpu, points: &[[f32; 2]]) -> Vec<Samples> {
    let parked = [5000.0, 5000.0];
    let mut race = Race::default();
    race.position = parked;
    let camera = Camera {
        zoom: ZOOM,
        tilt: 1.0,
        ..Camera::default()
    };
    let scene = TrackScene {
        race,
        camera,
        brake: 0.0,
    };
    let scene = Rc::new(RefCell::new(scene));
    let mut track = Track::new(Rc::clone(&scene), Rc::new(TrackLog::default()));
    mount(&mut track, gpu);
    let mut found = Vec::new();
    for point in points {
        scene.borrow_mut().camera.offset = [point[0] - parked[0], point[1] - parked[1]];
        found.push(sampled(&drawn(&mut track, gpu)));
    }
    found
}

/// The material's ground at each point: a plane under the racer's world
/// group, an orthographic camera above it spanning what the Track spans;
/// one renderer, mounted once, its camera moved between points.
fn by_material(gpu: &Gpu, points: &[[f32; 2]]) -> Vec<Samples> {
    let meshes = Rc::new(RefCell::new(Meshes::new()));
    let material = meshes
        .borrow_mut()
        .material(crate::ground_material())
        .expect("the ground reads only the contract");
    let mut scene = Scene::new();
    let world = scene
        .add(scene.root(), NodeKind::Group, racers_world())
        .unwrap();
    let kind = NodeKind::Mesh {
        geometry: plane(&mut meshes.borrow_mut()),
        colour: vapors_scene3d::Colour::srgb(1.0, 1.0, 1.0).unwrap(),
    };
    let ground = scene.add(world, kind, Transform::IDENTITY).unwrap();
    scene.set_material(ground, Some(material)).unwrap();
    let camera = camera(&mut scene);
    let scene = Rc::new(RefCell::new(scene));
    let mut renderer = SceneRenderer::new(Rc::clone(&scene), meshes);
    mount(&mut renderer, gpu);
    let mut found = Vec::new();
    for &point in points {
        scene
            .borrow_mut()
            .set_transform(camera, above(point))
            .unwrap();
        found.push(sampled(&drawn(&mut renderer, gpu)));
    }
    found
}

/// A ground plane 2000 units square around the circuit, in the racer's X, Y.
fn plane(meshes: &mut Meshes) -> GeometryHandle {
    let corners = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
    let positions = corners
        .map(|(x, y)| Vec3::new(x * 1000.0, y * 1000.0, 0.0))
        .to_vec();
    meshes
        .geometry(positions, vec![0, 1, 2, 0, 2, 3], vec![1.0; 4])
        .expect("a plane")
}

/// An orthographic camera in the scene's root, as high as the Track's
/// view at `ZOOM`, made active.
fn camera(scene: &mut Scene) -> NodeId {
    let handle = CameraHandle::new();
    let projection = Projection::orthographic(400.0 / ZOOM, 0.1, 2000.0).unwrap();
    let kind = NodeKind::Camera { handle, projection };
    let node = scene.add(scene.root(), kind, above([0.0, 0.0])).unwrap();
    scene.set_camera(Some(handle));
    node
}

/// Whether every channel of `found` is within 1 of `other`'s.
fn near(found: &Samples, other: &Samples) -> bool {
    let pairs = found.as_flattened().iter().zip(other.as_flattened());
    pairs.into_iter().all(|(a, b)| a.abs_diff(*b) <= 1)
}

/// Whether each sample is within its pin's tolerance.
fn pinned(found: &Samples, pins: &[Pin; 4]) -> bool {
    let within =
        |a: &[u8; 4], (pin, most): &Pin| a.iter().zip(pin).all(|(a, b)| a.abs_diff(*b) <= *most);
    found.iter().zip(pins).all(|(a, pin)| within(a, pin))
}

#[test]
fn gpu_the_ground_material_draws_the_tracks_colours_which_are_pinned() {
    // Given
    let gpu = gpu();
    let points: Vec<_> = POINTS.iter().map(|(_, point, _)| *point).collect();

    // When
    let track = by_track(&gpu, &points);
    let material = by_material(&gpu, &points);

    // Then: five centre colours, so each point is on its own part of the
    // ground; at all four samples the material within 1 a channel of the
    // Track, and the Track within each pin's tolerance
    let centres: std::collections::BTreeSet<_> = track.iter().map(|samples| samples[0]).collect();
    assert_eq!(centres.len(), 5, "{track:?}");
    for ((name, _, pins), (track, material)) in POINTS.iter().zip(track.iter().zip(&material)) {
        assert!(
            near(material, track),
            "{name}: material {material:?}, Track {track:?}"
        );
        assert!(
            pinned(track, pins),
            "{name}: Track {track:?}, pinned {pins:?}"
        );
    }
}
