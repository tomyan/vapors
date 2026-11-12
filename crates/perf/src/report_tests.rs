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
fn the_counts_line_gives_the_items_culled_after_the_scene_items() {
    // Given
    let mut frame = vapors_app::FrameCounts::default();
    frame.culled = 7;
    let samples = Samples {
        counts: Counts {
            frame,
            ..Counts::default()
        },
        ..Samples::default()
    };

    // When
    let report = section("rows", &samples);

    // Then
    assert!(report.contains("scene items 0, culled 7;"), "{report}");
}

#[test]
fn the_styles_line_gives_where_the_last_frames_styles_came_from() {
    // Given
    let mut update = vapors_elements::UpdateCounts::default();
    (update.styles_shared, update.styles_computed) = (12_000, 4);
    (
        update.styles_unkeyed,
        update.styles_unshareable,
        update.styles_live,
    ) = (2, 0, 6);
    let samples = Samples {
        counts: Counts {
            update,
            ..Counts::default()
        },
        ..Samples::default()
    };

    // When
    let report = section("rows", &samples);

    // Then
    assert!(
        report.contains(
            "Styles (last frame): shared 12000, computed 4, unkeyed 2, unshareable 0; live 6."
        ),
        "{report}"
    );
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

#[test]
fn the_gpu_times_resolution_is_reported_beside_them() {
    // Given
    let tick = std::time::Duration::from_nanos(83);
    let samples = Samples {
        gpu_time: Some(vec![std::time::Duration::from_micros(400)]),
        gpu_resolution: Some(vapors_app::GpuResolution::Tick(tick)),
        ..Samples::default()
    };

    // When
    let report = section("rows", &samples);

    // Then
    let row = "| GPU time resolution | 83ns (the device's timestamp period) | | |";
    assert!(report.contains(row), "{report}");
}

#[test]
fn the_times_section_has_no_gpu_rows() {
    // When
    let report = super::times_section("mount", &Samples::default(), &["update"]);

    // Then
    assert!(report.contains("| frame (CPU) |"), "{report}");
    assert!(!report.contains("GPU"), "{report}");
}

#[test]
fn the_counts_line_gives_the_batches_written_and_kept_and_the_bytes_kept() {
    // Given
    let mut frame = vapors_app::FrameCounts::default();
    (
        frame.batches_written,
        frame.batches_kept,
        frame.retained_bytes,
    ) = (2, 17, 2176);
    let samples = Samples {
        counts: Counts {
            frame,
            ..Counts::default()
        },
        ..Samples::default()
    };

    // When
    let report = section("rows", &samples);

    // Then
    let expected = "batches written 2, batches kept 17, retained bytes 2176, render passes";
    assert!(report.contains(expected), "{report}");
}
