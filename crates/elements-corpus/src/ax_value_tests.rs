//! Accessibility values by Chromium's value types.

use serde_json::json;

use super::*;

fn names() -> Related {
    Related::from_nodes(&[
        json!({ "nodeId": "7", "backendDOMNodeId": 70, "role": { "type": "role", "value": "StaticText" }, "name": { "type": "computedString", "value": "Brightness" } }),
        json!({ "nodeId": "8", "backendDOMNodeId": 80, "role": { "type": "role", "value": "heading" }, "name": { "type": "computedString", "value": "Layers" } }),
    ])
}

#[test]
fn scalar_values_are_written_bare() {
    for (value, text) in [
        (json!({ "type": "boolean", "value": true }), "true"),
        (json!({ "type": "tristate", "value": "mixed" }), "mixed"),
        (json!({ "type": "number", "value": 50 }), "50"),
        (json!({ "type": "computedString", "value": "Save" }), "Save"),
        (json!({ "type": "token", "value": "polite" }), "polite"),
        (json!({ "type": "valueUndefined" }), "undefined"),
    ] {
        assert_eq!(
            written(&value, &names(), "p"),
            Ok(text.to_owned()),
            "{value}"
        );
    }
}

#[test]
fn a_node_list_is_written_as_its_nodes_roles_and_names_never_their_ids() {
    // Given: aria-labelledby naming two elements
    let value = json!({ "type": "nodeList", "relatedNodes": [{ "backendDOMNodeId": 70, "idref": "label" }, { "backendDOMNodeId": 80 }] });

    // When / Then
    assert_eq!(
        written(&value, &names(), "labelledby"),
        Ok("StaticText \"Brightness\", heading \"Layers\"".to_owned())
    );
}

#[test]
fn a_value_of_a_type_not_read_or_a_related_node_not_in_the_tree_is_refused_naming_the_property() {
    for (value, needle) in [
        (
            json!({ "type": "domRelation", "value": 1 }),
            "controls: a value of type domRelation",
        ),
        (
            json!({ "type": "nodeList", "relatedNodes": [{ "backendDOMNodeId": 99 }] }),
            "controls: a related node 99",
        ),
        (json!({ "value": 1 }), "controls: a value with no type"),
    ] {
        let refusal = written(&value, &names(), "controls").expect_err(needle);
        assert!(refusal.starts_with(needle), "wanted {needle} in: {refusal}");
    }
}
