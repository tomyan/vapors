use super::*;
use vapors_scene3d::{Ambient, Colour, NodeKind, Quat, Transform};

fn colour(r: f32, g: f32, b: f32) -> Colour {
    Colour::srgb(r, g, b).unwrap()
}

fn words(bytes: &[u8]) -> Vec<u32> {
    let (chunks, _) = bytes.as_chunks::<4>();
    chunks.iter().map(|c| u32::from_le_bytes(*c)).collect()
}

fn floats(bytes: &[u8]) -> Vec<f32> {
    words(bytes).into_iter().map(f32::from_bits).collect()
}

/// Ambient, the sun turned a quarter turn about X so its -Z points down -Y,
/// a lamp at (1, 2, 3) and one child of a group at (0, 0, 4).
fn three_lights() -> Scene {
    let mut scene = Scene::new();
    scene.set_ambient(Ambient::new(colour(1.0, 1.0, 1.0), 0.5).unwrap());
    let root = scene.root();
    let sun = NodeKind::Light(Light::Directional {
        colour: colour(1.0, 0.0, 0.0),
        intensity: 2.0,
    });
    let turn = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    let tilt = Transform::new(Vec3::ZERO, turn, Vec3::ONE).unwrap();
    scene.add(root, sun, tilt).unwrap();
    let lamp = |intensity, range| {
        NodeKind::Light(Light::Point {
            colour: colour(0.0, 1.0, 0.0),
            intensity,
            range,
        })
    };
    let at = |x, y, z| Transform::from_translation(Vec3::new(x, y, z)).unwrap();
    scene.add(root, lamp(1.0, 10.0), at(1.0, 2.0, 3.0)).unwrap();
    let group = scene.add(root, NodeKind::Group, at(0.0, 0.0, 4.0)).unwrap();
    scene.add(group, lamp(0.5, 3.0), at(0.0, 1.0, 0.0)).unwrap();
    scene.update();
    scene
}

#[test]
fn the_bytes_hold_ambient_and_the_directional_light() {
    // Given / When
    let bytes = lights_bytes(&three_lights()).unwrap();

    // Then
    assert_eq!(bytes.len() as u64, LIGHTS_BYTES);
    let f = floats(&bytes);
    assert_eq!(f[..3], [0.5, 0.5, 0.5], "ambient");
    assert_eq!(words(&bytes)[4], 3, "count");
    assert_eq!(
        f[8..12],
        [2.0, 0.0, 0.0, 0.0],
        "sun colour times intensity, kind 0"
    );
    assert_eq!(f[12..16], [0.0; 4], "a directional has no position");
    assert!(f[16].abs() < 1e-6 && (f[17] + 1.0).abs() < 1e-6 && f[18].abs() < 1e-6);
}

#[test]
fn the_bytes_hold_two_point_lights_in_world_space_and_zeros_after() {
    // Given / When
    let bytes = lights_bytes(&three_lights()).unwrap();

    // Then
    let f = floats(&bytes);
    assert_eq!(f[20..23], [0.0, 1.0, 0.0], "lamp colour");
    assert_eq!(words(&bytes)[23], 1, "lamp kind");
    assert_eq!(f[24..28], [1.0, 2.0, 3.0, 10.0], "lamp position and range");
    assert_eq!(f[28..32], [0.0; 4], "a point has no direction");
    assert_eq!(
        f[32..35],
        [0.0, 0.5, 0.0],
        "child lamp colour times intensity"
    );
    assert_eq!(f[36..40], [0.0, 1.0, 4.0, 3.0], "child lamp in world space");
    assert!(f[44..].iter().all(|w| *w == 0.0), "unused lights are zeros");
}

#[test]
fn a_scene_without_lights_is_all_zeros() {
    // Given / When
    let mut scene = Scene::new();
    scene.update();

    // Then
    assert!(lights_bytes(&scene).unwrap().iter().all(|b| *b == 0));
}

#[test]
fn a_light_whose_world_position_overflows_is_refused_naming_it() {
    // Given: a point light 1e30 along a parent scaled by 1e30
    let mut scene = Scene::new();
    let root = scene.root();
    let big = Vec3::splat(1e30);
    let parent = Transform::new(Vec3::ZERO, Quat::IDENTITY, big).unwrap();
    let group = scene.add(root, NodeKind::Group, parent).unwrap();
    let lamp = NodeKind::Light(Light::Point {
        colour: colour(1.0, 1.0, 1.0),
        intensity: 1.0,
        range: 1.0,
    });
    let far = Transform::from_translation(big).unwrap();
    let node = scene.add(group, lamp, far).unwrap();
    scene.update();

    // When
    let refused = lights_bytes(&scene).unwrap_err();

    // Then
    assert_eq!(refused, FrameError::LightDegenerate(node));
}
