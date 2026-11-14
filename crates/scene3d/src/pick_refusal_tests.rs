//! What `pick` refuses rather than answering "nothing here".

use super::stage::{Stage, at, at_z, ortho, unit_square};
use crate::{
    CameraError, Meshes, PickAt, PickError, Quat, Scene, StoreError, Transform, Vec3, pick,
};

#[test]
fn a_scene_without_a_camera_is_refused() {
    // Given
    let at = PickAt {
        point: [1.0, 1.0],
        size: [10.0, 10.0],
    };

    // When
    let refused = pick(&mut Scene::new(), &Meshes::new(), at, |_| false);

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

/// A stage whose camera, 5 back, is scaled by `scale` on every axis.
fn scaled_camera(scale: f32) -> Stage {
    let eye = Transform::new(Vec3::new(0.0, 0.0, 5.0), Quat::IDENTITY, Vec3::splat(scale));
    let mut stage = Stage::with_eye(ortho(4.0), [100.0, 100.0], eye.unwrap());
    stage.square_at(at_z(0.0));
    stage
}

#[test]
fn a_camera_whose_view_projection_is_not_finite_is_refused_naming_it() {
    for scale in [1e-13, 1e13] {
        // Given: the camera's world matrix has no finite inverse
        let mut stage = scaled_camera(scale);

        // When
        let refused = stage.try_pick([50.0, 50.0]);

        // Then
        let expected = PickError::Camera(CameraError::NotFinite(stage.camera));
        assert_eq!(refused, Err(expected), "scale {scale}");
    }
}

#[test]
fn a_camera_whose_view_projection_has_no_finite_inverse_is_refused_naming_it() {
    // Given: a finite view-projection scaled by 1e-12, whose inverse is
    // not finite (measured: 1e13 already fails the forward matrix)
    let mut stage = scaled_camera(1e12);

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

#[test]
fn a_mesh_naming_geometry_its_meshes_does_not_hold_is_refused_naming_both() {
    // Given: geometry from another Meshes
    let mut stage = Stage::new();
    let foreign = unit_square(&mut Meshes::new(), 1.0);
    let geometry = foreign.id();
    let node = stage.mesh_at(foreign, at(Vec3::ZERO));

    // When
    let refused = stage.try_pick([50.0, 50.0]);

    // Then
    assert_eq!(refused, Err(PickError::GeometryNotHeld { node, geometry }));
}
