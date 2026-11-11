//! The report's counts line names what the last frame did.

use super::section;
use crate::run::{Counts, Samples};

#[test]
fn the_counts_line_gives_the_last_frames_render_passes() {
    // Given
    let samples = Samples {
        counts: Counts {
            render_passes: 3,
            ..Counts::default()
        },
        ..Samples::default()
    };

    // When
    let report = section("rows", &samples);

    // Then
    assert!(report.contains("render passes 3."), "{report}");
}
