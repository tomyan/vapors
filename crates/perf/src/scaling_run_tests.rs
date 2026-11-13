//! The phases `--scaling` takes from recorded samples.

use super::{at_size, by_kind, title};
use crate::instruction_samples::InstructionSamples;
use crate::report::{EVERY_PHASE, PHASES};
use crate::rows::RowChange;
use crate::run::Samples;
use crate::scaling::{Judged, Scaling};

fn samples(phases: &[&str]) -> Samples {
    counted(&phases.iter().map(|&p| (p, 5)).collect::<Vec<_>>())
}

fn counted(phases: &[(&str, u64)]) -> Samples {
    let mut kept = InstructionSamples::default();
    kept.push(10, phases.iter().map(|&(p, n)| (p.to_owned(), n)).collect());
    Samples {
        instructions: Some(kept),
        ..Samples::default()
    }
}

#[test]
fn every_recorded_phase_is_taken_with_the_frame() {
    // Given
    let samples = samples(&["update", "draw"]);

    // When
    let size = at_size(1000, &samples, &PHASES).expect("known phases");

    // Then
    let names: Vec<_> = size.phases.keys().map(String::as_str).collect();
    assert_eq!(names, ["draw", "frame (CPU)", "update"]);
}

#[test]
fn the_apps_other_frame_phases_are_judged_with_the_rest() {
    // Given: the five phases `run_frame` spans around no work
    let samples = samples(&[
        "update",
        "input",
        "tasks",
        "tick",
        "accessibility",
        "record",
    ]);

    // When
    let size = at_size(1000, &samples, &EVERY_PHASE).expect("known phases");

    // Then
    let names: Vec<_> = size.phases.keys().map(String::as_str).collect();
    let judged = [
        "accessibility",
        "frame (CPU)",
        "input",
        "record",
        "tasks",
        "tick",
        "update",
    ];
    assert_eq!(names, judged);
}

#[test]
fn an_app_phase_that_grows_with_the_page_is_a_violation() {
    // Given: tick flat at 1000 rows, 6000 instructions up at 10,000
    let small = at_size(
        1000,
        &counted(&[("update", 100), ("tick", 100)]),
        &EVERY_PHASE,
    );
    let large = at_size(
        10_000,
        &counted(&[("update", 100), ("tick", 6100)]),
        &EVERY_PHASE,
    );
    let sizes = [small.expect("known"), large.expect("known")];

    // When
    let scaling = Scaling::of(&sizes).expect("two sizes");
    let violations = scaling.violations(Judged::EveryPhase).expect("judged");

    // Then
    assert_eq!(violations.len(), 1, "{violations:?}");
    assert!(violations[0].starts_with("tick grows"), "{violations:?}");
}

#[test]
fn a_recorded_phase_the_judge_does_not_know_is_refused_naming_it() {
    // Given
    let samples = samples(&["update", "mystery"]);

    // When
    let error = at_size(1000, &samples, &EVERY_PHASE).expect_err("unknown phase");

    // Then
    assert!(error.contains("phase mystery"), "{error}");
    assert!(error.contains("report::PHASES"), "{error}");
}

#[test]
fn a_first_frame_phase_is_known_to_the_mount_list_and_not_to_the_steady_table() {
    // Given
    let samples = samples(&["update", "accessibility"]);

    // When
    let steady = at_size(1000, &samples, &PHASES);
    let first = at_size(1000, &samples, &crate::mount_report::MOUNT_PHASES);

    // Then
    assert!(steady.expect_err("steady").contains("accessibility"));
    assert!(first.is_ok());
}

/// Frames alternating `first` and `second` instructions in `update`.
fn alternating(first: u64, second: u64, frames: usize) -> Samples {
    let mut kept = InstructionSamples::default();
    for frame in 0..frames {
        let count = if frame % 2 == 0 { first } else { second };
        kept.push(count, [("update".to_owned(), count)].into());
    }
    Samples {
        instructions: Some(kept),
        ..Samples::default()
    }
}

fn update_medians(sizes: &[crate::scaling::AtSize]) -> Vec<u64> {
    sizes.iter().map(|size| size.phases["update"]).collect()
}

#[test]
fn alternating_kinds_get_a_median_each_in_the_order_the_change_makes_them() {
    // Given: an even warmup, so the first kept frame is the first kind
    let samples = alternating(100, 900, 6);

    // When
    let sizes = by_kind(1000, &samples, 2, 10).expect("counted");

    // Then: one kind's median no longer stands for both
    assert_eq!(update_medians(&sizes), [100, 900]);
}

#[test]
fn an_odd_warmup_makes_the_first_kept_frame_the_second_kind() {
    // Given: frame 11 is the first kept, a frame of the second kind
    let samples = alternating(900, 100, 6);

    // When
    let sizes = by_kind(1000, &samples, 2, 11).expect("counted");

    // Then
    assert_eq!(update_medians(&sizes), [100, 900]);
}

#[test]
fn a_change_of_one_kind_takes_the_median_of_every_frame() {
    // Given
    let samples = alternating(100, 900, 5);

    // When
    let sizes = by_kind(1000, &samples, 1, 10).expect("counted");

    // Then
    assert_eq!(update_medians(&sizes), [100]);
}

#[test]
fn a_kinds_title_names_the_kind_only_when_the_change_makes_more_than_one() {
    assert_eq!(title(RowChange::Insert, "remove"), "Insert, remove frames");
    assert_eq!(title(RowChange::Text, "every"), "Text");
}
