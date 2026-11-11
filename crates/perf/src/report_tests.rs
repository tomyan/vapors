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

#[test]
fn the_gpu_rows_are_the_pass_time_and_the_span_labelled_an_upper_bound() {
    // Given
    let samples = Samples {
        gpu: Some(vec![std::time::Duration::from_micros(900)]),
        gpu_time: Some(vec![std::time::Duration::from_micros(400)]),
        ..Samples::default()
    };

    // When
    let report = section("rows", &samples);

    // Then
    assert!(report.contains("| GPU time | 400.0 |"), "{report}");
    assert!(
        report.contains("| GPU span (upper bound) | 900.0 |"),
        "{report}"
    );
}

#[test]
fn without_timestamp_queries_both_gpu_rows_say_so() {
    let report = section("rows", &Samples::default());
    assert!(
        report.contains("| GPU time | no timestamp queries |"),
        "{report}"
    );
    assert!(
        report.contains("| GPU span (upper bound) | no timestamp queries |"),
        "{report}"
    );
}
