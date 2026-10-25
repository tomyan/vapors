//! Chromium's accessibility tree, reduced to what the oracle commits.

use serde_json::{Value, json};

use super::*;

fn node(id: &str, role: &str, name: &str, children: &[&str]) -> Value {
    json!({ "nodeId": id, "ignored": false, "role": { "type": "role", "value": role }, "name": { "type": "computedString", "value": name }, "childIds": children })
}

#[test]
fn the_tree_is_listed_in_order_with_depth_role_name_and_properties() {
    // Given
    let mut button = node("2", "button", "Save", &[]);
    button["properties"] = json!([
        { "name": "focusable", "value": { "type": "booleanOrUndefined", "value": true } },
        { "name": "checked", "value": { "type": "tristate", "value": "mixed" } }
    ]);
    let nodes = json!({ "nodes": [node("1", "RootWebArea", "Card", &["2"]), button] });

    // When
    let tree = reduce(&nodes).expect("reduces");

    // Then
    assert_eq!(
        tree,
        json!([
            { "depth": 0, "role": "RootWebArea", "name": "Card" },
            { "depth": 1, "role": "button", "name": "Save", "properties": ["checked=mixed", "focusable=true"] }
        ])
    );
}

#[test]
fn ignored_nodes_and_inline_text_boxes_are_left_out_and_their_children_kept() {
    // Given
    let mut ignored = node("2", "generic", "", &["3"]);
    ignored["ignored"] = json!(true);
    let nodes = json!({ "nodes": [
        node("1", "RootWebArea", "", &["2"]),
        ignored,
        node("3", "StaticText", "Hello", &["4"]),
        node("4", "InlineTextBox", "Hello", &[]),
    ] });

    // When
    let tree = reduce(&nodes).expect("reduces");

    // Then
    assert_eq!(
        tree,
        json!([
            { "depth": 0, "role": "RootWebArea" },
            { "depth": 1, "role": "StaticText", "name": "Hello" }
        ])
    );
}

#[test]
fn a_tree_of_another_shape_is_refused_naming_what_is_missing() {
    for (nodes, needle) in [
        (json!({}), "nodes"),
        (json!({ "nodes": [] }), "no root"),
        (
            json!({ "nodes": [{ "ignored": false, "childIds": [] }] }),
            "nodeId",
        ),
        (
            json!({ "nodes": [node("1", "RootWebArea", "", &["9"])] }),
            "9",
        ),
    ] {
        let refusal = reduce(&nodes).expect_err(needle);
        assert!(refusal.contains(needle), "wanted {needle} in: {refusal}");
    }
}

#[test]
fn the_page_url_is_left_out_since_it_names_a_temporary_file() {
    // Given
    let mut root = node("1", "RootWebArea", "", &[]);
    root["properties"] = json!([
        { "name": "url", "value": { "type": "string", "value": "file:///tmp/vapors-ax-oracle-1/a.html" } },
        { "name": "focusable", "value": { "type": "booleanOrUndefined", "value": true } }
    ]);

    // When
    let tree = reduce(&json!({ "nodes": [root] })).expect("reduces");

    // Then
    assert_eq!(tree[0]["properties"], json!(["focusable=true"]));
}

#[test]
fn a_tree_whose_structure_is_wrong_is_refused_naming_how() {
    let root = |children: &[&str]| node("1", "RootWebArea", "", children);
    let mut roleless = node("2", "button", "", &[]);
    roleless.as_object_mut().expect("object").remove("role");
    for (nodes, needle) in [
        (
            json!({ "nodes": [root(&["2"]), node("2", "button", "", &[]), node("2", "link", "", &[])] }),
            "two a11y nodes with id 2",
        ),
        (
            json!({ "nodes": [root(&["2"]), roleless] }),
            "a11y node 2 has no role",
        ),
        (
            json!({ "nodes": [root(&["2"]), node("2", "group", "", &["1"])] }),
            "a11y node 1 is reached twice",
        ),
        (
            json!({ "nodes": [root(&[]), node("2", "button", "", &[])] }),
            "1 a11y node is not under the root",
        ),
    ] {
        let refusal = reduce(&nodes).expect_err(needle);
        assert!(refusal.contains(needle), "wanted {needle} in: {refusal}");
    }
}

#[test]
fn value_and_description_are_kept_and_an_empty_name_left_out() {
    // Given
    let mut slider = node("2", "slider", "", &[]);
    slider["value"] = json!({ "type": "number", "value": 40 });
    slider["description"] = json!({ "type": "computedString", "value": "Brightness of the layer" });
    let nodes = json!({ "nodes": [node("1", "RootWebArea", "", &["2"]), slider] });

    // When
    let tree = reduce(&nodes).expect("reduces");

    // Then
    assert_eq!(
        tree[1],
        json!({ "depth": 1, "role": "slider", "value": "40", "description": "Brightness of the layer" })
    );
}
