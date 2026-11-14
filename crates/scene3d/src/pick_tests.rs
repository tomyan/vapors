//! Which mesh wins: depth, skipping, visibility, transforms, ties.

use super::stage::{Stage, at_z, unit_square};
use crate::{Colour, Projection, Quat, Transform, Vec3};

#[test]
fn the_nearest_mesh_under_the_point_is_picked_not_one_behind_it() {
    // Given: a square at z 0 behind one at z 1, both under the centre
    let mut stage = Stage::new();
    let _behind = stage.square_at(at_z(0.0));
    let front = stage.square_at(at_z(1.0));

    // Then
    assert_eq!(stage.pick([50.0, 50.0], |_| false), Some(front));
}

#[test]
fn a_skipped_mesh_lets_the_pick_through_to_the_one_behind() {
    // Given
    let mut stage = Stage::new();
    let behind = stage.square_at(at_z(0.0));
    let front = stage.square_at(at_z(1.0));

    // Then
    assert_eq!(stage.pick([50.0, 50.0], |node| node == front), Some(behind));
}

#[test]
fn a_hidden_mesh_is_not_picked() {
    // Given
    let mut stage = Stage::new();
    let behind = stage.square_at(at_z(0.0));
    let front = stage.square_at(at_z(1.0));
    stage
        .scene
        .set_appearance(front, Colour::BLACK, false)
        .unwrap();

    // Then
    assert_eq!(stage.pick([50.0, 50.0], |_| false), Some(behind));
}

#[test]
fn a_mesh_seen_edge_on_is_missed_and_its_transform_is_followed() {
    // Given: the square turned a quarter about +Y, and one moved right
    let mut stage = Stage::new();
    let edge_on = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    stage.square_at(Transform::new(Vec3::ZERO, edge_on, Vec3::ONE).unwrap());
    let right = stage.square_at(Transform::from_translation(Vec3::X).unwrap());

    // Then: 1 unit right is pixel 75
    assert_eq!(stage.pick([45.0, 50.0], |_| false), None);
    assert_eq!(stage.pick([75.0, 50.0], |_| false), Some(right));
}

#[test]
fn meshes_at_exactly_the_same_depth_pick_the_first_in_draw_order() {
    // Given: two coplanar squares; the renderer draws the first first,
    // and its Less depth test keeps it
    let mut stage = Stage::new();
    let first = stage.square_at(at_z(0.0));
    let _second = stage.square_at(at_z(0.0));

    // Then
    assert_eq!(stage.pick([50.0, 50.0], |_| false), Some(first));
}

#[test]
fn nearness_compares_across_differently_scaled_and_turned_meshes() {
    // Given: a square shrunk by half in front of one grown four times
    // and turned: in their own spaces the ray is twice and a quarter as
    // long, so comparing local distances would pick the one behind
    let mut stage = Stage::new();
    let turned = Quat::from_rotation_z(0.7);
    let _behind = stage.square_at(Transform::new(Vec3::ZERO, turned, Vec3::splat(4.0)).unwrap());
    let half = Vec3::splat(0.5);
    let front = stage.square_at(Transform::new(Vec3::Z, Quat::IDENTITY, half).unwrap());

    // Then
    assert_eq!(stage.pick([50.0, 50.0], |_| false), Some(front));
}

#[test]
fn a_tiny_mesh_under_a_matching_camera_is_picked() {
    // Given: a square of side 2e-4 filling half of a view 4e-4 high
    // whose near-to-far segment is 1 long (250,000 pixels a unit; the
    // square covers pixels 25 to 75); an absolute parallel threshold
    // (f32::EPSILON) misses it
    let projection = Projection::orthographic(4e-4, 0.0, 1.0).unwrap();
    let mut stage = Stage::viewed(projection, [100.0, 100.0]);
    let tiny = unit_square(&mut stage.meshes, 2e-4);
    let square = stage.mesh_at(tiny, at_z(4.5));

    // Then
    assert_eq!(stage.pick([70.0, 50.0], |_| false), Some(square));
    assert_eq!(stage.pick([80.0, 50.0], |_| false), None);
}
