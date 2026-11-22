//! A capture is refused before any browser starts when what it would
//! capture is not a paint fixture at a scale.

use super::*;

/// A scratch directory for what a capture would write, so a refusal that
/// regresses writes nothing into `fixtures/references/`.
fn scratch() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("vapors-capture-")
        .tempdir()
        .expect("a scratch dir")
}

#[test]
fn a_capture_of_a_fixture_that_is_not_a_paint_fixture_is_refused() {
    // When: borders has no `/* paint */` in its style
    let dir = scratch();
    let refused = capture_into(dir.path(), "borders", 2).expect_err("refused");

    // Then
    assert!(
        refused.contains("borders is not a paint fixture"),
        "{refused}"
    );
}

#[test]
fn a_capture_at_scale_zero_is_refused() {
    // When
    let dir = scratch();
    let refused = capture_into(dir.path(), "text_line_box_paint", 0).expect_err("refused");

    // Then
    assert!(refused.contains("scale 0"), "{refused}");
}
