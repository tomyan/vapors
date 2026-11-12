//! The phases `--scaling` takes from recorded samples.

use super::at_size;
use crate::instruction_samples::InstructionSamples;
use crate::run::Samples;

fn samples(phases: &[&str]) -> Samples {
    let mut kept = InstructionSamples::default();
    kept.push(10, phases.iter().map(|&p| (p.to_owned(), 5)).collect());
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
    let size = at_size(1000, &samples).expect("known phases");

    // Then
    let names: Vec<_> = size.phases.keys().map(String::as_str).collect();
    assert_eq!(names, ["draw", "frame (CPU)", "update"]);
}

#[test]
fn the_apps_other_frame_phases_are_left_out() {
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
    let size = at_size(1000, &samples).expect("known phases");

    // Then
    let names: Vec<_> = size.phases.keys().map(String::as_str).collect();
    assert_eq!(names, ["frame (CPU)", "update"]);
}

#[test]
fn a_recorded_phase_the_judge_does_not_know_is_refused_naming_it() {
    // Given
    let samples = samples(&["update", "mystery"]);

    // When
    let error = at_size(1000, &samples).expect_err("unknown phase");

    // Then
    assert!(error.contains("phase mystery"), "{error}");
    assert!(error.contains("report::PHASES"), "{error}");
}
