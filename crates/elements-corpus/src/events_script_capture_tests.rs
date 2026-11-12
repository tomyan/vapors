//! The `.events` directives for pointer capture (3-E1): `capture
//! <target>…`, the elements that capture on `pointerdown`, and one
//! finger's `touch-down`, `touch-move` and `touch-up`.

use super::*;

const SCRIPT: &str = "\
listen pointerdown pointermove pointerup gotpointercapture
capture div#thumb div#knob    both capture on pointerdown
press 10 10
expect event gotpointercapture div#thumb
touch-down 10 10
expect event pointerdown div#thumb
touch-move 10 50
expect event pointermove div#thumb
touch-up 10 50
expect event pointerup div#thumb
";

#[test]
fn capture_names_the_elements_that_capture_on_pointerdown() {
    // When
    let script = parse("drag", SCRIPT).expect("parses");

    // Then
    assert_eq!(script.capture, ["div#thumb", "div#knob"]);
}

#[test]
fn a_finger_goes_down_moves_and_lifts() {
    // When
    let script = parse("drag", SCRIPT).expect("parses");

    // Then
    let actions: Vec<&Action> = script.steps.iter().map(|step| &step.action).collect();
    assert_eq!(
        actions[1..],
        [
            &Action::TouchDown(10.0, 10.0),
            &Action::TouchMove(10.0, 50.0),
            &Action::TouchUp(10.0, 50.0)
        ]
    );
}

#[test]
fn a_second_capture_or_one_without_targets_is_refused() {
    // When
    let twice = parse("drag", "listen click\ncapture div#a\ncapture div#b\n").expect_err("twice");
    let empty = parse("drag", "listen click\ncapture\n").expect_err("empty");

    // Then
    assert!(
        twice.contains("drag.events:3: a second `capture`"),
        "{twice}"
    );
    assert!(
        empty.contains("drag.events:2: `capture` takes targets"),
        "{empty}"
    );
}
