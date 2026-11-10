//! GPU tier: what `ScenePainter` refuses about surfaces it would
//! otherwise have to approximate (docs/gpu-surface.md): a backing larger
//! than the device allows, two surfaces sharing one name and an
//! unmounted surface, each also when the surface is clipped out of view;
//! a renderer's GPU validation error; what a refused frame releases; and
//! `draw_over` refusing as `draw` does.

#[path = "support/surface.rs"]
mod support;

use support::{BLUE, Misfit, RED, Rig, Solid, bare_document, document, update};
use vapors_app::{DrawOverError, PaintError};
use vapors_elements::Document;

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

/// A 0 px tall panel that clips, holding `surfaces`, each 8 px square
/// unless `css` sizes it: every surface is clipped out of view.
fn clipped_out(css: &str, surfaces: usize) -> Document {
    let panel = ".panel { height: 0; overflow: hidden; } gpu-surface { width: 8px; height: 8px; } ";
    let mut elements = vec![(None, "div", "panel")];
    elements.extend((0..surfaces).map(|_| (Some(0), "gpu-surface", "track")));
    let mut doc = document(&format!("{panel}{css}"), &elements);
    update(&mut doc, 1.0);
    doc
}

#[test]
fn gpu_an_unmounted_surface_clipped_out_of_view_is_refused() {
    // Given
    let doc = clipped_out("", 1);
    let mut rig = Rig::new();

    // When
    let refused = rig.draw(&doc).expect_err("refused");

    // Then
    assert!(
        matches!(refused, PaintError::Unmounted(ref name) if name == "track"),
        "{refused:?}"
    );
}

#[test]
fn gpu_a_surface_clipped_out_of_view_is_refused_when_shared_or_too_large() {
    for (css, surfaces, expected) in [
        ("", 2, "DuplicateName"),
        (".track { width: 40000px; }", 1, "TooLarge"),
    ] {
        // Given
        let doc = clipped_out(css, surfaces);
        let mut rig = Rig::new();
        let (solid, log) = Solid::new(RED);
        rig.painter.mount("track", Box::new(solid)).expect("mounts");

        // When
        let refused = rig.draw(&doc).expect_err("refused");

        // Then
        assert!(format!("{refused:?}").starts_with(expected), "{refused:?}");
        assert_eq!(log.borrow().resizes.len(), 0, "{css}");
    }
}

#[test]
fn gpu_a_mounted_surface_clipped_out_of_view_is_not_rendered_and_holds_no_backing() {
    // Given
    let doc = clipped_out("", 1);
    let mut rig = Rig::new();
    let (solid, log) = Solid::new(RED);
    rig.painter.mount("track", Box::new(solid)).expect("mounts");
    rig.painter.invalidate("track").expect("mounted");

    // When
    rig.draw(&doc).expect("draws");

    // Then
    let log = log.borrow();
    assert_eq!((log.mounts, log.resizes.len(), log.frames.len()), (0, 0, 0));
    assert!(!rig.painter.wants_frame());
}

#[test]
fn gpu_a_renderer_gpu_validation_error_refuses_its_frames_until_remounted() {
    // Given
    let mut doc = document(
        ".track { width: 8px; height: 8px; }",
        &[(None, "gpu-surface", "track")],
    );
    update(&mut doc, 1.0);
    let mut rig = Rig::new();
    rig.painter
        .mount("track", Box::new(Misfit::default()))
        .expect("mounts");

    // When
    let refused = rig.draw(&doc).expect_err("refused");

    // Then
    assert!(
        matches!(refused, PaintError::Renderer { ref surface, .. } if surface == "track"),
        "{refused:?}"
    );
    assert!(refused.to_string().contains("Bgra8Unorm"), "{refused}");
    rig.draw(&doc).expect_err("refused until remounted");
    rig.painter.unmount("track").expect("mounted");
    let solid = Box::new(Solid::new(RED).0);
    rig.painter.mount("track", solid).expect("mounts");
    rig.draw(&doc).expect("a remounted renderer draws");
}

#[test]
fn gpu_a_refused_frame_releases_the_backings_it_did_not_paint() {
    // Given
    let css = "gpu-surface { display: block; width: 8px; height: 8px; }";
    let elements = [
        (None, "gpu-surface", "left"),
        (None, "gpu-surface", "right"),
    ];
    let mut doc = document(css, &elements);
    update(&mut doc, 1.0);
    let mut rig = Rig::new();
    let (right, log) = Solid::new(BLUE);
    rig.painter
        .mount("left", Box::new(Solid::new(RED).0))
        .expect("mounts");
    rig.painter.mount("right", Box::new(right)).expect("mounts");
    rig.draw(&doc).expect("draws");

    // When
    let left = rig.painter.unmount("left").expect("mounted");
    rig.draw(&doc).expect_err("refused: left is unmounted");
    rig.painter.mount("left", left).expect("mounts");
    rig.draw(&doc).expect("draws");

    // Then
    assert_eq!(log.borrow().resizes.len(), 2);
}

#[test]
fn gpu_draw_over_refuses_a_surface_refusal_as_a_paint_error() {
    // Given
    let mut doc = bare_document(
        "gpu-surface { width: 8px; height: 8px; }",
        &[(None, "gpu-surface", "track")],
    );
    update(&mut doc, 1.0);
    let mut rig = Rig::new();

    // When
    let refused = rig.draw_over(&doc).expect_err("refused");

    // Then
    assert!(
        matches!(refused, DrawOverError::Paint(PaintError::Unmounted(ref name)) if name == "track"),
        "{refused:?}"
    );
}
