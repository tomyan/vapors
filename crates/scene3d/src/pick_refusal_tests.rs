//! What `pick` refuses rather than answering "nothing here".

use super::stage::{Stage, at_z, ortho};
use crate::{CameraError, PickAt, PickError, Quat, Scene, StoreError, Transform, Vec3, pick};

#[test]
fn a_scene_without_a_camera_is_refused() {
    // Given
    let at = PickAt {
        point: [1.0, 1.0],
        size: [10.0, 10.0],
    };

    // When
    let refused = pick(&mut Scene::new(), at, |_| false);

    // Then
    let expected = PickError::Camera(CameraError::Store(StoreError::NoCamera));
    assert_eq!(refused, Err(expected));
}

#[test]
fn a_surface_size_not_finite_and_positive_is_refused_naming_it() {
    for size in [
        [-100.0, -100.0],
        [0.0, 100.0],
        [100.0, f32::NAN],
        [f32::INFINITY, 100.0],
    ] {
        // Given
        let mut stage = Stage::viewed(ortho(4.0), size);
        stage.square_at(at_z(0.0));

        // When
        let refused = stage.try_pick([50.0, 50.0]);

        // Then
        let Err(PickError::SizeNotPositive(named)) = refused else {
            panic!("{size:?} gave {refused:?}");
        };
        assert_eq!(format!("{named:?}"), format!("{size:?}"));
    }
}

#[test]
fn a_point_not_finite_is_refused_naming_it() {
    for point in [[f32::NAN, 50.0], [50.0, f32::NEG_INFINITY]] {
        // Given
        let mut stage = Stage::new();
        stage.square_at(at_z(0.0));

        // When
        let refused = stage.try_pick(point);

        // Then
        let Err(PickError::PointNotFinite(named)) = refused else {
            panic!("{point:?} gave {refused:?}");
        };
        assert_eq!(format!("{named:?}"), format!("{point:?}"));
    }
}

#[test]
fn an_aspect_the_projection_refuses_is_refused() {
    // Given: a surface so wide its aspect overflows
    let mut stage = Stage::viewed(ortho(4.0), [3e38, 1e-3]);

    // When
    let refused = stage.try_pick([1.0, 1e-4]);

    // Then
    assert!(
        matches!(refused, Err(PickError::Camera(CameraError::Projection(_)))),
        "{refused:?}"
    );
}

/// A stage whose rigid camera stands `along` world units along +X and 5
/// back, through an orthographic lens `height` tall.
fn camera_at(along: f32, height: f32) -> Stage {
    let eye = Transform::from_translation(Vec3::new(along, 0.0, 5.0)).unwrap();
    let mut stage = Stage::with_eye(ortho(height), [100.0, 100.0], eye);
    stage.square_at(at_z(0.0));
    stage
}

#[test]
fn a_camera_whose_view_projection_is_not_finite_is_refused_naming_it() {
    // Given: a camera 3e38 along X through a lens 1e-3 tall, whose
    // view-projection's translation overflows (2000 * 3e38); a camera's
    // scale, which reached this before, is refused as not rigid
    let mut stage = camera_at(3e38, 1e-3);

    // When
    let refused = stage.try_pick([50.0, 50.0]);

    // Then
    let expected = PickError::Camera(CameraError::NotFinite(stage.camera));
    assert_eq!(refused, Err(expected));
}

#[test]
fn a_camera_whose_view_projection_has_no_finite_inverse_is_refused_naming_it() {
    // Given: a lens 1e30 tall, a finite view-projection whose determinant
    // underflows f32, so its inverse is not finite
    let mut stage = camera_at(0.0, 1e30);

    // When
    let refused = stage.try_pick([50.0, 50.0]);

    // Then
    assert_eq!(refused, Err(PickError::ViewNotInvertible(stage.camera)));
}

#[test]
fn a_visible_mesh_with_no_finite_inverse_is_refused_naming_it() {
    for scale in [1e-20, 1e-13] {
        // Given
        let mut stage = Stage::new();
        let flat = Transform::new(Vec3::ZERO, Quat::IDENTITY, Vec3::splat(scale)).unwrap();
        let mesh = stage.square_at(flat);

        // When
        let refused = stage.try_pick([50.0, 50.0]);

        // Then
        assert_eq!(
            refused,
            Err(PickError::MeshNotInvertible(mesh)),
            "scale {scale}"
        );
    }
}
