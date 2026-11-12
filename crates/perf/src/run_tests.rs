//! Samples keep each phase's times aligned with the frames, a phase a
//! frame did not enter counting 0 for it.

use std::collections::BTreeMap;
use std::time::Duration;

use super::Samples;

fn phases(entries: &[(&str, u64)]) -> BTreeMap<String, Duration> {
    let micros = |&(name, time): &(&str, u64)| (name.to_owned(), Duration::from_micros(time));
    entries.iter().map(micros).collect()
}

#[test]
fn a_phase_missing_from_a_frame_counts_zero_for_it() {
    // Given
    let mut samples = Samples::default();

    // When
    samples.push(Duration::from_micros(10), phases(&[("draw", 4)]));
    samples.push(Duration::from_micros(20), phases(&[("update", 5)]));
    samples.push(Duration::from_micros(30), phases(&[("draw", 6)]));

    // Then
    let micros = |phase: &str| -> Vec<u128> {
        samples.phases[phase]
            .iter()
            .map(Duration::as_micros)
            .collect()
    };
    assert_eq!(micros("draw"), [4, 0, 6]);
    assert_eq!(micros("update"), [0, 5, 0]);
    assert_eq!(samples.frame.len(), 3);
}
