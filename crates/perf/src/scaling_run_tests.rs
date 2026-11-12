//! The phases `--scaling` takes from recorded samples.

use super::at_size;
use crate::instruction_samples::InstructionSamples;
use crate::report::{EVERY_PHASE, PHASES};
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
