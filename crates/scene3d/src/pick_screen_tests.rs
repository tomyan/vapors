//! Where a surface point casts its ray: +Y down in pixels, the aspect,
//! perspective, and points off the surface. Expected pixels are worked
//! out by hand from the stage's numbers, not by the code under test.

use super::stage::{Stage, at, at_z};
use crate::{Projection, Quat, Transform, Vec3};

#[test]
fn a_point_outside_every_mesh_picks_nothing() {
    // Given: 25 pixels a unit, so the square covers 37.5 to 62.5
    let mut stage = Stage::new();
    stage.square_at(at_z(0.0));

    // Then
    assert_eq!(stage.pick([30.0, 50.0], |_| false), None);
    assert!(
        stage.pick([40.0, 45.0], |_| false).is_some(),
        "inside, off centre"
    );
}

#[test]
fn a_mesh_above_the_centre_is_picked_above_it_on_the_surface() {
    // Given: a square 1 unit up, 25 pixels above the centre (y 25), as
    // surface y grows downwards
    let mut stage = Stage::new();
    let up = stage.square_at(at(Vec3::Y));

    // Then
    assert_eq!(stage.pick([50.0, 25.0], |_| false), Some(up));
    assert_eq!(stage.pick([50.0, 75.0], |_| false), None);
}

#[test]
fn a_wide_surface_shows_more_width_at_the_same_scale() {
    // Given: 200 x 100, 4 units high, so 8 units wide at 25 pixels a
    // unit with the origin at pixel 100; a square 2 right is at pixel 150
    let mut stage = Stage::viewed(super::stage::ortho(4.0), [200.0, 100.0]);
    let right = stage.square_at(at(Vec3::new(2.0, 0.0, 0.0)));

    // Then
    assert_eq!(stage.pick([150.0, 50.0], |_| false), Some(right));
    assert_eq!(stage.pick([100.0, 50.0], |_| false), None);
}

#[test]
fn a_perspective_camera_picks_along_its_rays() {
    // Given: a vertical field of view of 1 radian, the camera at z 5 and
    // a unit square at x 1, z -2, 7 in front of it. Its x 0.5 to 1.5
    // projects to NDC x / (7 tan 0.5): 0.1307 to 0.3922, centre 0.2617;
    // as pixels, (1 + ndc) * 50: 56.5 to 69.6, centre 63.1
    let projection = Projection::perspective(1.0, 0.1, 100.0).unwrap();
    let mut stage = Stage::viewed(projection, [100.0, 100.0]);
    let off = stage.square_at(at(Vec3::new(1.0, 0.0, -2.0)));

    // Then
    assert_eq!(stage.pick([63.1, 50.0], |_| false), Some(off));
    assert_eq!(stage.pick([55.0, 50.0], |_| false), None);
    assert_eq!(stage.pick([71.0, 50.0], |_| false), None);
}

#[test]
fn a_point_off_the_surface_picks_nothing_and_its_edges_are_on_it() {
    // Given: a square grown to cover the whole view
    let mut stage = Stage::new();
    let scale = Vec3::splat(10.0);
    let all = stage.square_at(Transform::new(Vec3::ZERO, Quat::IDENTITY, scale).unwrap());

    // Then
    for off in [[-1.0, 50.0], [101.0, 50.0], [50.0, -0.5], [50.0, 100.5]] {
        assert_eq!(stage.try_pick(off), Ok(None), "{off:?}");
    }
    for edge in [[0.0, 0.0], [100.0, 100.0]] {
        assert_eq!(stage.try_pick(edge), Ok(Some(all)), "{edge:?}");
    }
}
