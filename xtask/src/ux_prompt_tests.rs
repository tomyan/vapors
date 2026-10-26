//! The prompt holds the judged items only, and measured items must name
//! recorded facts.

use serde_json::json;

use super::*;
use crate::ux_checklist::parse;
use crate::ux_run::Frame;

const CHECKLIST: &str = "- measured smallTargets: Targets are at least 44 px.\n- measured tapsToFirstEdit: Few taps.\n- judged first-screen: The first screen says what to do.\n";

fn phone() -> Viewport {
    Viewport {
        name: "phone".to_owned(),
        facts: json!({ "tapsToFirstEdit": 7, "steps": [{ "facts": { "smallTargets": [] } }] }),
        frames: vec![Frame {
            name: "phone/01-first-open.png".to_owned(),
            path: "p".into(),
            size: (412, 915),
            text: "Open a photo".to_owned(),
        }],
    }
}

#[test]
fn the_prompt_lists_judged_items_frames_and_design_but_not_measured_items() {
    // Given
    let items = parse("c.md", CHECKLIST).expect("parsed");
    let design = [("design/patterns.md".to_owned(), "Bottom sheet".to_owned())];

    // When
    let text = text(&items, &[phone()], &design);

    // Then
    assert!(text.contains("- first-screen: The first screen says what to do."));
    assert!(!text.contains("Targets are at least 44 px"));
    assert!(text.contains("## Design file design/patterns.md\nBottom sheet"));
    assert!(text.contains("### phone/01-first-open.png\nVisible text:\nOpen a photo"));
}

#[test]
fn measured_items_found_at_the_top_or_in_a_step_pass() {
    // Given
    let items = parse("c.md", CHECKLIST).expect("parsed");

    // When
    let checked = measured_present("c.md", &items, &[phone()]);

    // Then
    assert_eq!(checked, Ok(()));
}

#[test]
fn a_measured_item_naming_no_recorded_fact_is_refused_with_its_line() {
    // Given
    let text = format!("{CHECKLIST}- measured contrast: Text is legible.\n");
    let items = parse("c.md", &text).expect("parsed");

    // When
    let refused = measured_present("c.md", &items, &[phone()]).expect_err("refused");

    // Then
    assert!(
        refused
            .starts_with("c.md:4: the measured item `contrast` names no fact in phone/facts.json"),
        "{refused}"
    );
}

#[test]
fn a_measured_item_naming_a_structural_key_is_refused() {
    // Given
    let text = format!("{CHECKLIST}- measured steps: The walk has steps.\n");
    let items = parse("c.md", &text).expect("parsed");

    // When
    let refused = measured_present("c.md", &items, &[phone()]).expect_err("refused");

    // Then
    assert!(
        refused.contains("the measured item `steps` names no fact"),
        "{refused}"
    );
}
