//! The checklist's items read, and its other shapes refused.

use super::*;

const GOOD: &str = "# UX checklist\n\nProse is ignored.\n\n- measured smallTargets: Touch targets are at least 44 px.\n- judged first-screen: The first screen says what to do first.\n";

#[test]
fn measured_and_judged_items_are_read_with_their_lines() {
    // When
    let items = parse("design/ux-checklist.md", GOOD).expect("parsed");

    // Then
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].kind, Kind::Measured);
    assert_eq!(items[0].id, "smallTargets");
    assert_eq!(items[1].kind, Kind::Judged);
    assert_eq!(items[1].line, 6);
}

#[test]
fn a_list_item_of_another_shape_is_refused_naming_file_and_line() {
    // Given
    let text = format!("{GOOD}- check the colours\n");

    // When
    let refused = parse("design/ux-checklist.md", &text).expect_err("refused");

    // Then
    assert!(
        refused.starts_with("design/ux-checklist.md:7: "),
        "{refused}"
    );
}

#[test]
fn an_item_with_no_text_or_a_bad_id_is_refused() {
    for line in [
        "- judged first screen: two words",
        "- judged first-screen: ",
        "- judged first-screen",
    ] {
        // Given
        let text = format!("{GOOD}{line}\n");

        // When
        let refused = parse("c.md", &text);

        // Then
        assert!(refused.is_err(), "{line}");
    }
}

#[test]
fn a_repeated_id_is_refused_naming_both_lines() {
    // Given
    let text = format!("{GOOD}- judged first-screen: again\n");

    // When
    let refused = parse("c.md", &text).expect_err("refused");

    // Then
    assert!(
        refused.contains("c.md:7: the id `first-screen` is already used on line 6"),
        "{refused}"
    );
}

#[test]
fn a_checklist_with_nothing_to_judge_is_refused() {
    // Given
    let text = "- measured smallTargets: Touch targets are at least 44 px.\n";

    // When
    let refused = parse("c.md", text).expect_err("refused");

    // Then
    assert!(refused.contains("nothing to ask the model"), "{refused}");
}

#[test]
fn a_list_item_of_any_other_markdown_shape_is_refused_naming_its_line() {
    for line in [
        "* judged a: b",
        "+ judged a: b",
        "1. judged a: b",
        "  - judged a: b",
    ] {
        // Given
        let text = format!("{GOOD}{line}\n");

        // When
        let refused = parse("c.md", &text).expect_err(line);

        // Then
        assert!(
            refused.starts_with("c.md:7: a checklist item starts `- ` at column 0"),
            "{refused}"
        );
    }
}

#[test]
fn list_items_in_fenced_code_are_not_read() {
    // Given
    let text = format!("{GOOD}```\n- not an item\n```\n");

    // When
    let items = parse("c.md", &text).expect("parsed");

    // Then
    assert_eq!(items.len(), 2);
}
