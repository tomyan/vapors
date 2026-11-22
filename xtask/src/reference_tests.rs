//! The reference command runs the corpus's capture.

use super::*;

#[test]
fn the_reference_command_runs_the_corpus_capture_with_the_fixture_and_scale() {
    // When
    let steps = steps("text_line_box_paint", 2);

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
            "text_line_box_paint",
            "2"
        ]
    );
}
