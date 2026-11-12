//! GPU tier: a refused frame's `frame_counts` hold the work issued
//! before the refusal, including the refused run's own (IU1 PR 3).

#[path = "support/surface.rs"]
mod support;

use support::{RED, Rig, Solid, bare_document, update};
use vapors_app::PaintError;
use vapors_elements::Document;

/// Two 8 px square surfaces `a` and `b` share one run; only `a` is mounted.
fn only_a_mounted() -> (Document, Rig) {
    let css = "gpu-surface { width: 8px; height: 8px; }";
    let elements = [(None, "gpu-surface", "a"), (None, "gpu-surface", "b")];
    let mut doc = bare_document(css, &elements);
    update(&mut doc, 1.0);
    let mut rig = Rig::new();
    rig.painter
        .mount("a", Box::new(Solid::new(RED).0))
        .expect("mounts");
    (doc, rig)
}

#[test]
fn gpu_a_refused_run_counts_the_work_issued_before_its_refusal() {
    // Given
    let (doc, mut rig) = only_a_mounted();

    // When
    let refused = rig.draw(&doc).expect_err("b is unmounted");
    let counts = rig.painter.frame_counts();

    // Then
    assert!(matches!(refused, PaintError::Unmounted(ref name) if name == "b"));
    assert_eq!(counts.surfaces.draw_calls, 1);
    assert_eq!(counts.total().draw_calls, 1);
}

#[test]
fn gpu_the_next_frame_starts_from_zero_after_a_refused_one() {
    // Given
    let (doc, mut rig) = only_a_mounted();
    rig.draw(&doc).expect_err("b is unmounted");
    rig.painter
        .mount("b", Box::new(Solid::new(RED).0))
        .expect("mounts");

    // When
    rig.draw(&doc).expect("draws");

    // Then
    assert_eq!(rig.painter.frame_counts().surfaces.draw_calls, 2);
}

#[test]
fn gpu_a_refused_frame_counts_the_bind_groups_still_kept() {
    // Given
    let (doc, mut rig) = only_a_mounted();

    // When
    rig.draw(&doc).expect_err("b is unmounted");

    // Then: `a`'s backing keeps its bind group through the refusal
    assert_eq!(rig.painter.frame_counts().bind_groups_kept, 1);
}
