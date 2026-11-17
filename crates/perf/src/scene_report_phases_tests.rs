//! The scene run's phase table: each phase's median and 99th percentile,
//! in frame order, a phase a frame did not enter counted as 0.

use std::collections::BTreeMap;
use std::time::Duration;

use super::phases_table;
use crate::sampler::JudgedFrame;

/// 100 frames whose `layout` takes 1..=100 µs; `surface` in odd frames
/// only, at 10 µs.
fn frames() -> Vec<JudgedFrame> {
    (1..=100_u64)
        .map(|n| {
            let mut phases = BTreeMap::from([("layout".to_owned(), Duration::from_micros(n))]);
            if n % 2 == 1 {
                phases.insert("surface".to_owned(), Duration::from_micros(10));
            }
            JudgedFrame {
                phases,
                ..JudgedFrame::default()
            }
        })
        .collect()
}

#[test]
fn each_phase_has_its_median_and_p99_and_one_not_entered_counts_as_0() {
    // Given
    let mut out = String::new();

    // When
    phases_table(&mut out, &frames());

    // Then
    assert!(out.contains("| layout | 50.0 | 99.0 |"), "{out}");
    assert!(out.contains("| surface | 0.0 | 10.0 |"), "{out}");
    assert!(out.contains("| submit | 0.0 | 0.0 |"), "{out}");
}

#[test]
fn the_phases_come_in_frame_order() {
    // Given
    let mut out = String::new();

    // When
    phases_table(&mut out, &frames());

    // Then
    let at = |phase: &str| out.find(&format!("| {phase} |")).expect(phase);
    assert!(
        at("input") < at("layout") && at("layout") < at("draw"),
        "{out}"
    );
    assert!(
        at("draw") < at("surface") && at("surface") < at("submit"),
        "{out}"
    );
}
