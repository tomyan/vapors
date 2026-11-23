//! The reference command runs the corpus's capture with its target.

use super::*;

#[test]
fn the_reference_command_passes_its_target_to_the_corpus_capture() {
    // When
    let target = ["paint_boxes", "2", "card_radius_4"].map(String::from);
    let steps = steps(&target);

    // Then
    let [step] = steps.as_slice() else {
        panic!("one step, got {}", steps.len())
    };
    assert_eq!(step.program, "cargo");
    assert_eq!(
        step.args,
        [
            "run",
            "-p",
            "vapors-elements-corpus",
            "--example",
            "paint_reference",
            "--",
            "paint_boxes",
            "2",
            "card_radius_4"
        ]
    );
}
