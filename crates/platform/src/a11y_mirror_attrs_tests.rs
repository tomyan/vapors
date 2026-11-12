//! A record's attributes follow r6.md's element mapping; a record the
//! mapping has no attribute for is refused naming the node.

use vapors_a11y::{AxMirrorNode, AxRange, AxRect};

use super::a11y_mirror_attrs::{attributes, text};
use super::a11y_mirror_tests::node;

/// The attributes set, as `name=value`, sorted.
fn set(node: &AxMirrorNode) -> Vec<String> {
    let mut set: Vec<String> = attributes(node)
        .expect("mapped")
        .into_iter()
        .filter_map(|(name, value)| value.map(|value| format!("{name}={value}")))
        .collect();
    set.sort();
    set
}

#[test]
fn a_slider_carries_its_value_and_range_and_a_box_in_its_parent() {
    // Given
    let mut slider = node(5, None, "slider", "Radius");
    slider.value = Some("12".to_owned());
    slider.range = Some(AxRange {
        min: 0.0,
        max: 50.0,
    });
    slider.focusable = true;
    slider.bounds = AxRect {
        x: 10.5,
        y: 20.0,
        width: 100.0,
        height: 24.0,
    };

    // Then
    assert_eq!(
        set(&slider),
        [
            "aria-label=Radius",
            "aria-valuemax=50",
            "aria-valuemin=0",
            "aria-valuenow=12",
            "data-node=5",
            "role=slider",
            "style=position:absolute;left:10.5px;top:20px;width:100px;height:24px",
            "tabindex=-1",
        ]
    );
}

#[test]
fn a_text_field_value_is_its_text_and_an_option_is_selected_false_by_default() {
    // Given
    let mut field = node(6, None, "textbox", "Prompt");
    field.value = Some("warm".to_owned());
    let option = node(7, None, "option", "Mono");

    // Then
    assert_eq!(text(&field), "warm");
    assert_eq!(text(&node(8, None, "button", "Go")), "");
    assert!(set(&field).iter().all(|a| !a.starts_with("aria-value")));
    assert!(set(&option).contains(&"aria-selected=false".to_owned()));
}

#[test]
fn states_and_a_description_are_aria_attributes_and_live_roles_are_off() {
    // Given
    let mut check = node(9, None, "checkbox", "Mono");
    check.states.checked = Some(false);
    check.states.disabled = true;
    check.states.expanded = Some(true);
    check.description = Some("Grey only".to_owned());
    let alert = node(10, None, "alert", "Saved");

    // Then
    assert_eq!(
        set(&check),
        [
            "aria-checked=false",
            "aria-description=Grey only",
            "aria-disabled=true",
            "aria-expanded=true",
            "aria-label=Mono",
            "data-node=9",
            "role=checkbox",
            "style=position:absolute;left:0px;top:0px;width:0px;height:0px",
        ]
    );
    assert!(set(&alert).contains(&"aria-live=off".to_owned()));
}

#[test]
fn a_ranged_role_without_a_value_or_range_is_refused_naming_it() {
    // Given
    let mut slider = node(5, None, "spinbutton", "Amount");
    slider.range = Some(AxRange { min: 0.0, max: 9.0 });

    // When
    let refused = attributes(&slider).expect_err("no value");

    // Then
    assert!(
        refused.contains("spinbutton \"Amount\" has no value"),
        "{refused}"
    );
    slider.value = Some("1".to_owned());
    slider.range = None;
    let refused = attributes(&slider).expect_err("no range");
    assert!(refused.contains("has no range"), "{refused}");
}

#[test]
fn a_value_on_a_role_the_mapping_has_no_attribute_for_is_refused() {
    // Given
    let mut button = node(5, None, "button", "Go");
    button.value = Some("3".to_owned());

    // When
    let refused = attributes(&button).expect_err("no attribute");

    // Then
    assert!(
        refused.contains("button \"Go\" carries a value"),
        "{refused}"
    );
}
