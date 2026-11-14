//! The 60 s run's report: its statistics, its first frames, and a line
//! per frame over budget.

use std::time::Duration;

use super::{over_budget_lines, report};
use crate::sampler::JudgedFrame;
use crate::scene_kind::SceneKind;

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
    let report = report(&frames(), 0.42, Some(0.61), SceneKind::Racer);
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
    let report = report(&frames(), 0.0, None, SceneKind::Racer);
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
    let lines = over_budget_lines(&[&frame], SceneKind::Racer);

    // Then
    assert!(
        lines.starts_with("frame 99 at 1.584 s, changes []: to idle 20000.0 µs"),
        "{lines}"
    );
    assert!(lines.contains("phases (µs) update 1500.0"), "{lines}");
}

#[test]
fn a_run_of_no_frames_reports_no_samples_in_both_tables() {
    let report = report(&[], 0.0, None, SceneKind::Racer);
    assert!(
        report.contains("| frame to idle | no samples | no samples | no samples |"),
        "{report}"
    );
    assert!(
        report.contains("| draw calls | no samples | no samples |"),
        "{report}"
    );
}

#[test]
fn a_one_scene_frame_is_printed_with_its_scripted_changes_and_the_scenes_work() {
    // Given: frame 125 of the one-scene run, a click on light 1
    let mut frame = frames().remove(99);
    frame.index = 125;
    frame.counts.frame.surfaces.draw_calls = 5;
    frame.counts.frame.surfaces.buffer_bytes = 240;

    // When
    let lines = over_budget_lines(&[&frame], SceneKind::OneScene);

    // Then
    assert!(
        lines.starts_with("frame 125 at 2.000 s, changes [click light 1]:"),
        "{lines}"
    );
    assert!(
        lines.contains("surfaces: draw calls 5, buffer bytes 240"),
        "{lines}"
    );
}

#[test]
fn the_report_lists_the_ten_slowest_frames_even_when_none_is_over_budget() {
    // Given: frame 99 at 20 ms made 15 ms, within budget
    let mut frames = frames();
    frames[99].idle = Duration::from_millis(15);

    // When
    let report = report(&frames, 0.0, None, SceneKind::OneScene);

    // Then: frames 99 down to 90, slowest first
    assert!(report.contains("### one-scene example, 60 s"), "{report}");
    let slowest = report
        .split("The 10 slowest frames to idle:")
        .nth(1)
        .expect("the table");
    let order: Vec<&str> = slowest
        .lines()
        .filter_map(|line| line.split(" at ").next())
        .filter(|f| f.starts_with("frame "))
        .collect();
    assert_eq!(order.first(), Some(&"frame 99"), "{slowest}");
    assert_eq!(order.len(), 10, "{slowest}");
    assert_eq!(order.last(), Some(&"frame 90"), "{slowest}");
}
