//! The a11y projection the events oracle compares (5-A1's start): for
//! each node with a role the script names, in tree order, its role, its
//! name, its value and its description when it has them, and the properties a control's
//! state shows: `expanded`, `controls`, `activedescendant`, `selected`,
//! `focused`, `checked`, `disabled` and `pressed`, in the oracle's
//! reduced shape (`ax_tree`: related nodes as role and name).
//! Depth is left out: another page's wrappers differ from a fixture's.

use serde_json::{Value, json};

/// The properties kept.
pub const KEPT: [&str; 8] = [
    "activedescendant",
    "checked",
    "controls",
    "disabled",
    "expanded",
    "focused",
    "pressed",
    "selected",
];

/// The projection of a reduced tree (`ax_tree::reduce`) to `roles`.
///
/// # Errors
/// A tree of another shape: not a list, or a node without a role.
pub fn project(reduced: &Value, roles: &[String]) -> Result<Value, String> {
    let nodes = reduced
        .as_array()
        .ok_or("the reduced a11y tree is not a list")?;
    let mut out = Vec::new();
    for node in nodes {
        let role = node["role"]
            .as_str()
            .ok_or_else(|| format!("an a11y node without a role: {node}"))?;
        if roles.iter().any(|kept| kept == role) {
            out.push(entry(node, role));
        }
    }
    Ok(Value::Array(out))
}

/// One node: role, name, value and description when it has them, and
/// the kept properties, sorted.
fn entry(node: &Value, role: &str) -> Value {
    let properties: Vec<&str> = node["properties"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|property| {
            KEPT.iter().any(|kept| {
                property
                    .split_once('=')
                    .is_some_and(|(name, _)| name == *kept)
            })
        })
        .collect();
    let mut entry = json!({ "role": role, "name": node["name"].as_str().unwrap_or(""), "properties": properties });
    for key in ["value", "description"] {
        if let Some(text) = node[key].as_str() {
            entry[key] = json!(text);
        }
    }
    entry
}

#[cfg(test)]
#[path = "ax_projection_tests.rs"]
mod tests;
