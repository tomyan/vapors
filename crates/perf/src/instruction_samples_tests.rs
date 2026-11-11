//! Instruction samples keep each phase's counts aligned with the frames.

use std::collections::BTreeMap;

use super::InstructionSamples;

fn phases(entries: &[(&str, u64)]) -> BTreeMap<String, u64> {
    entries
        .iter()
        .map(|&(name, n)| (name.to_owned(), n))
        .collect()
}

#[test]
fn a_phase_missing_from_a_frame_counts_zero_for_it() {
    // Given
    let mut samples = InstructionSamples::default();

    // When
    samples.push(10, phases(&[("draw", 4)]));
    samples.push(12, phases(&[("update", 5)]));

    // Then
    assert_eq!(samples.frame, [10, 12]);
    assert_eq!(samples.phases["draw"], [4, 0]);
    assert_eq!(samples.phases["update"], [0, 5]);
}
