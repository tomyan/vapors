//! States the mirror cannot write truthfully are refused, named (R6,
//! review 229-H1): a `checkbox` role with no checked state (ARIA
//! requires `aria-checked` there, and a reader says "not checked" for
//! none), a `mixed` checked or pressed state, which the tree reads as
//! none, and a focusable `separator` without `aria-valuenow`.

mod support;

use support::mirror::page_with;
use vapors_elements::{AxError, Document};

/// The role, name and need of a refused take.
fn refused(mut doc: Document) -> (String, String, String) {
    match doc.take_ax_update() {
        Err(AxError::Unexposable { role, name, needs }) => (role, name, needs),
        other => panic!("not refused: {other:?}"),
    }
}

#[test]
fn a_checkbox_role_without_a_checked_state_is_refused() {
    // Given
    let doc = page_with("checkbox", &[("aria-label", "Bare")]);

    // When
    let (role, name, needs) = refused(doc);

    // Then
    assert_eq!((role.as_str(), name.as_str()), ("checkbox", "Bare"));
    assert!(needs.contains("aria-checked"), "{needs}");
}

#[test]
fn a_checkbox_role_with_a_checked_state_is_exposed() {
    // Given
    let mut doc = page_with(
        "checkbox",
        &[("aria-label", "Bare"), ("aria-checked", "false")],
    );

    // When
    let update = doc.take_ax_update().expect("exposed").expect("every");

    // Then
    assert_eq!(update.nodes[0].states.checked, Some(false));
}

#[test]
fn a_mixed_checked_state_is_refused_naming_mixed() {
    // Given
    let doc = page_with(
        "checkbox",
        &[("aria-label", "Some"), ("aria-checked", "mixed")],
    );

    // When
    let (role, _, needs) = refused(doc);

    // Then
    assert_eq!(role, "checkbox");
    assert!(
        needs.contains("aria-checked") && needs.contains("mixed"),
        "{needs}"
    );
}

#[test]
fn a_mixed_pressed_state_is_refused_naming_mixed() {
    // Given
    let doc = page_with(
        "button",
        &[("aria-label", "Bold"), ("aria-pressed", "mixed")],
    );

    // When
    let (role, _, needs) = refused(doc);

    // Then
    assert_eq!(role, "button");
    assert!(
        needs.contains("aria-pressed") && needs.contains("mixed"),
        "{needs}"
    );
}

#[test]
fn a_focusable_separator_without_a_value_is_refused() {
    // Given
    let doc = page_with("separator", &[("aria-label", "Split"), ("tabindex", "0")]);

    // When
    let (role, name, needs) = refused(doc);

    // Then
    assert_eq!((role.as_str(), name.as_str()), ("separator", "Split"));
    assert!(needs.contains("aria-valuenow"), "{needs}");
}

#[test]
fn a_focusable_separator_with_a_value_is_exposed() {
    // Given
    let mut doc = page_with(
        "separator",
        &[
            ("aria-label", "Split"),
            ("tabindex", "0"),
            ("aria-valuenow", "40"),
        ],
    );

    // When
    let update = doc.take_ax_update().expect("exposed").expect("every");

    // Then
    assert_eq!(update.nodes[0].role, "separator");
    assert!(update.nodes[0].focusable);
}

#[test]
fn a_separator_that_takes_no_focus_needs_no_value() {
    // Given
    let mut doc = page_with("separator", &[("aria-label", "Split")]);

    // When
    let update = doc.take_ax_update().expect("exposed").expect("every");

    // Then
    assert_eq!(update.nodes[0].role, "separator");
    assert!(!update.nodes[0].focusable);
}
