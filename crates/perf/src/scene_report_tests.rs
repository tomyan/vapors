//! The 60 s run's report: its statistics, its first frames, and a line
//! per frame over budget.

use std::time::Duration;

use super::{over_budget_lines, report};
use crate::sampler::JudgedFrame;

/// 100 frames: CPU 1..=100 µs, GPU twice that, to idle three times,
/// and frame 99's to idle 20 ms.
fn frames() -> Vec<JudgedFrame> {
    let mut frames: Vec<JudgedFrame> = (1..=100)
        .map(|n| JudgedFrame {
            index: usize::try_from(n - 1).expect("small"),
            cpu: Duration::from_micros(n),
            gpu: Duration::from_micros(2 * n),
            idle: Duration::from_micros(3 * n),
            ..JudgedFrame::default()
        })
        .collect();
    frames[99].idle = Duration::from_millis(20);
    frames
}

#[test]
fn the_report_gives_median_p99_and_max_of_each_time_and_the_over_budget_count() {
    let report = report(&frames(), 0.42, Some(0.61));
    assert!(
        report.contains("| frame (CPU) | 50.0 | 99.0 | 100.0 |"),
        "{report}"
    );
    assert!(
        report.contains("| GPU time | 100.0 | 198.0 | 200.0 |"),
        "{report}"
    );
    assert!(
        report.contains("| frame to idle | 150.0 | 297.0 | 20000.0 |"),
        "{report}"
    );
    assert!(report.contains("over 16.7 ms to idle: 1."), "{report}");
    assert!(
        report.contains("at the start 0.42, at the end 0.61"),
        "{report}"
    );
}

#[test]
fn the_report_prints_the_first_20_frames_on_their_own() {
    let report = report(&frames(), 0.0, None);
    assert!(report.contains("| 0 | 1.0 | 2.0 | 3.0 |"), "{report}");
    assert!(report.contains("| 19 | 20.0 | 40.0 | 60.0 |"), "{report}");
    assert!(!report.contains("| 20 | 21.0 |"), "{report}");
    assert!(report.contains("at the end unread."), "{report}");
}

#[test]
fn a_frame_over_budget_is_printed_with_its_time_on_the_clock_and_its_phases() {
    // Given
    let mut frame = frames().remove(99);
    frame
        .phases
        .insert("update".into(), Duration::from_micros(1500));

    // When
    let lines = over_budget_lines(&[&frame]);

    // Then
    assert!(
        lines.starts_with("frame 99 at 1.584 s: to idle 20000.0 µs"),
        "{lines}"
    );
    assert!(lines.contains("phases (µs) update 1500.0"), "{lines}");
}

#[test]
fn a_run_of_no_frames_reports_no_samples_in_both_tables() {
    let report = report(&[], 0.0, None);
    assert!(
        report.contains("| frame to idle | no samples | no samples | no samples |"),
        "{report}"
    );
    assert!(
        report.contains("| draw calls | no samples | no samples |"),
        "{report}"
    );
}
