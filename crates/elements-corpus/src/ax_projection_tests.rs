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
