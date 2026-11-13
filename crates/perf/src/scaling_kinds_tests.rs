//! A median per kind of frame, and the refusals around it.

use super::{MIN_FRAMES_PER_KIND, by_kind, enough_frames, per_kind, title};
use crate::instruction_samples::InstructionSamples;
use crate::report::EVERY_PHASE;
use crate::rows::RowChange;
use crate::run::Samples;
use crate::scaling_run::at_size;

fn samples(phases: &[&str]) -> Samples {
    let mut kept = InstructionSamples::default();
    kept.push(10, phases.iter().map(|&p| (p.to_owned(), 5)).collect());
    Samples {
        instructions: Some(kept),
        ..Samples::default()
    }
}

/// Frames alternating `first` and `second` instructions in `update`,
/// after `warmup` frames.
fn alternating(first: u64, second: u64, frames: usize, warmup: usize) -> Samples {
    let mut kept = InstructionSamples::default();
    for frame in 0..frames {
        let count = if frame % 2 == 0 { first } else { second };
        kept.push(count, [("update".to_owned(), count)].into());
    }
    Samples {
        instructions: Some(kept),
        warmup,
        ..Samples::default()
    }
}

fn update_medians(sizes: &[crate::scaling::AtSize]) -> Vec<u64> {
    sizes.iter().map(|size| size.phases["update"]).collect()
}

#[test]
fn alternating_kinds_get_a_median_each_in_the_order_the_change_makes_them() {
    // Given: an even warmup, so the first kept frame is the first kind
    let samples = alternating(100, 900, 6, 10);

    // When
    let sizes = by_kind(1000, &samples, &["insert", "remove"]).expect("counted");

    // Then: one kind's median no longer stands for both
    assert_eq!(update_medians(&sizes), [100, 900]);
}

#[test]
fn an_odd_warmup_makes_the_first_kept_frame_the_second_kind() {
    // Given: frame 11 is the first kept, a frame of the second kind
    let samples = alternating(900, 100, 6, 11);

    // When
    let sizes = by_kind(1000, &samples, &["insert", "remove"]).expect("counted");

    // Then
    assert_eq!(update_medians(&sizes), [100, 900]);
}

#[test]
fn a_change_of_one_kind_takes_the_median_of_every_frame() {
    // Given
    let samples = alternating(100, 900, 5, 10);

    // When
    let sizes = by_kind(1000, &samples, &["every"]).expect("counted");

    // Then
    assert_eq!(update_medians(&sizes), [100]);
}

#[test]
fn a_kinds_title_names_the_kind_only_when_the_change_makes_more_than_one() {
    assert_eq!(title(RowChange::Insert, "remove"), "Insert, remove frames");
    assert_eq!(title(RowChange::Text, "every"), "Text");
}

#[test]
fn a_kind_with_no_kept_frame_is_refused_naming_the_kind() {
    // Given: one kept frame, an insert
    let samples = alternating(100, 900, 1, 10);

    // When
    let error = by_kind(1000, &samples, &["insert", "remove"]).expect_err("no remove frame");

    // Then
    assert!(error.starts_with("remove frames: "), "{error}");
}

#[test]
fn too_few_runs_for_each_kind_are_refused_naming_the_change_and_the_command() {
    // Given
    let least = 2 * MIN_FRAMES_PER_KIND;

    // When
    let error = enough_frames(RowChange::Insert, least - 1).expect_err("too few");

    // Then
    assert!(error.contains("insert, remove"), "{error}");
    assert!(
        error.contains(&format!("vapors-perf {least} 10 --scaling Insert")),
        "{error}"
    );
    assert!(enough_frames(RowChange::Insert, least).is_ok());
    assert!(enough_frames(RowChange::Text, MIN_FRAMES_PER_KIND).is_ok());
}

#[test]
fn a_size_measured_with_the_wrong_number_of_kinds_is_refused() {
    // Given: Insert makes two kinds; the second size has one
    let size = |rows| at_size(rows, &samples(&["update"]), &EVERY_PHASE).expect("known");
    let measured = vec![vec![size(1000), size(1000)], vec![size(3000)]];

    // When
    let error = per_kind(RowChange::Insert, measured)
        .err()
        .expect("one kind short");

    // Then
    assert!(error.contains("measured with 1"), "{error}");
}
