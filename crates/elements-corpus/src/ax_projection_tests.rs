//! The projection keeps the named roles, in order, with their kept
//! properties.

use serde_json::json;

use super::*;

#[test]
fn only_the_named_roles_and_kept_properties_remain_in_order() {
    // Given
    let reduced = json!([
        { "depth": 0, "role": "RootWebArea", "properties": ["focusable=true"] },
        { "depth": 3, "role": "combobox", "name": "State", "properties": ["activedescendant=option \"Alaska\"", "autocomplete=list", "expanded=true", "focused=true", "hasPopup=listbox"] },
        { "depth": 2, "role": "StaticText", "name": "State" },
        { "depth": 4, "role": "option", "name": "Alaska", "properties": ["focusable=true", "selected=true"] },
    ]);
    let roles = vec!["combobox".to_owned(), "option".to_owned()];

    // When
    let projected = project(&reduced, &roles).expect("projects");

    // Then
    assert_eq!(
        projected,
        json!([
            { "role": "combobox", "name": "State", "properties": ["activedescendant=option \"Alaska\"", "expanded=true", "focused=true"] },
            { "role": "option", "name": "Alaska", "properties": ["selected=true"] },
        ])
    );
}

#[test]
fn a_node_without_a_role_is_refused() {
    assert!(project(&json!([{ "depth": 0 }]), &["option".to_owned()]).is_err());
}

#[test]
fn a_controls_states_and_value_are_kept() {
    // Given: a disabled, checked checkbox, a pressed toggle button and a slider
    let reduced = json!([
        { "depth": 1, "role": "checkbox", "name": "Lock", "properties": ["checked=true", "disabled=true", "invalid=false"] },
        { "depth": 1, "role": "button", "name": "Pin", "properties": ["focusable=true", "pressed=true"] },
        { "depth": 1, "role": "slider", "name": "Amount", "value": "50", "properties": ["valuetext=50"] },
    ]);
    let roles = vec![
        "checkbox".to_owned(),
        "button".to_owned(),
        "slider".to_owned(),
    ];

    // When
    let projected = project(&reduced, &roles).expect("projects");

    // Then
    assert_eq!(
        projected,
        json!([
            { "role": "checkbox", "name": "Lock", "properties": ["checked=true", "disabled=true"] },
            { "role": "button", "name": "Pin", "properties": ["pressed=true"] },
            { "role": "slider", "name": "Amount", "value": "50", "properties": [] },
        ])
    );
}

#[test]
fn a_description_is_kept() {
    // Given: a button its content names, described by its title
    let reduced = json!([
        { "depth": 1, "role": "button", "name": "Open", "description": "Open (o)", "properties": ["focusable=true"] },
    ]);

    // When
    let projected = project(&reduced, &["button".to_owned()]).expect("projects");

    // Then
    assert_eq!(
        projected,
        json!([{ "role": "button", "name": "Open", "description": "Open (o)", "properties": [] }])
    );
}
