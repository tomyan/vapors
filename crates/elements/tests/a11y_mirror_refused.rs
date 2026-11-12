//! A node the mirror cannot expose truthfully is refused, named (R6):
//! ARIA requires a state vapors' tree does not carry, so the browser
//! would read a default that may be wrong. The refusal names the role,
//! the name and what is missing.

mod support;

use support::mirror::page_with;
use vapors_elements::AxError;

#[test]
fn a_heading_without_a_level_is_refused_naming_it() {
    // Given
    let mut doc = page_with("heading", &[("aria-label", "Levels")]);

    // When
    let refused = doc.take_ax_update();

    // Then
    match refused {
        Err(AxError::Unexposable { role, name, needs }) => {
            assert_eq!((role.as_str(), name.as_str()), ("heading", "Levels"));
            assert!(needs.contains("aria-level"), "{needs}");
        }
        other => panic!("not refused: {other:?}"),
    }
}

#[test]
fn a_radio_without_a_checked_state_is_refused() {
    // Given
    let mut doc = page_with("radio", &[("aria-label", "Mono")]);

    // When
    let refused = doc.take_ax_update();

    // Then
    let message = refused.expect_err("refused").to_string();
    assert!(
        message.contains("radio") && message.contains("aria-checked"),
        "{message}"
    );
}

#[test]
fn a_radio_with_a_checked_state_is_exposed() {
    // Given
    let mut doc = page_with("radio", &[("aria-label", "Mono"), ("aria-checked", "true")]);

    // When
    let update = doc.take_ax_update().expect("exposed").expect("every");

    // Then
    assert_eq!(update.nodes[0].role, "radio");
    assert_eq!(update.nodes[0].states.checked, Some(true));
}

#[test]
fn an_option_is_exposed_with_its_selection_either_way() {
    // Given: aria-selected is required on an option; absent is false
    let mut doc = page_with("option", &[("aria-label", "Mono")]);

    // When
    let update = doc.take_ax_update().expect("exposed").expect("every");

    // Then
    assert_eq!(update.nodes[0].role, "option");
    assert!(!update.nodes[0].states.selected);
}
