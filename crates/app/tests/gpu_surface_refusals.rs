//! GPU tier: what `ScenePainter` refuses about surfaces it would
//! otherwise have to approximate (docs/gpu-surface.md): a backing larger
//! than the device allows, and two surfaces sharing one name.

#[path = "support/surface.rs"]
mod support;

use support::{RED, Rig, Solid, document, update};
use vapors_app::PaintError;

#[test]
fn gpu_a_surface_larger_than_the_device_allows_is_refused_naming_the_limit() {
    // Given
    let css = ".track { width: 40000px; height: 16px; }";
    let mut doc = document(css, &[(None, "gpu-surface", "track")]);
    update(&mut doc, 1.0);
    let mut rig = Rig::new();
    let limit = rig.gpu.device.limits().max_texture_dimension_2d;
    let (solid, log) = Solid::new(RED);
    rig.painter.mount("track", Box::new(solid)).expect("mounts");

    // When
    let refused = rig.draw(&doc).expect_err("refused");

    // Then
    assert!(
        matches!(refused, PaintError::TooLarge { ref surface, width: 40000, limit: l, .. } if surface == "track" && l == limit),
        "{refused:?}"
    );
    assert!(
        refused.to_string().contains(&limit.to_string()),
        "{refused}"
    );
    assert_eq!(log.borrow().resizes.len(), 0);
}

#[test]
fn gpu_two_surfaces_with_one_name_are_refused() {
    // Given
    let css = "gpu-surface { width: 8px; height: 8px; }";
    let elements = [
        (None, "gpu-surface", "track"),
        (None, "gpu-surface", "track"),
    ];
    let mut doc = document(css, &elements);
    update(&mut doc, 1.0);
    let mut rig = Rig::new();
    rig.painter
        .mount("track", Box::new(Solid::new(RED).0))
        .expect("mounts");

    // When
    let refused = rig.draw(&doc).expect_err("refused");

    // Then
    assert!(
        matches!(refused, PaintError::DuplicateName(ref name) if name == "track"),
        "{refused:?}"
    );
}
