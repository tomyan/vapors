//! Accessibility values, written by Chromium's value type (`AXValue.type`):
//! scalars bare, and the node types (`aria-labelledby`, `aria-controls`,
//! `aria-owns`) as the related nodes' roles and names, never their ids,
//! which differ from run to run. A type the oracle does not read, or a
//! related node not in the tree, is refused, naming the property.

use std::collections::BTreeMap;

use serde_json::Value;

/// Types written as their value.
const SCALARS: [&str; 11] = [
    "boolean",
    "tristate",
    "booleanOrUndefined",
    "integer",
    "number",
    "string",
    "computedString",
    "token",
    "tokenList",
    "role",
    "internalRole",
];

/// Types whose value is the related nodes.
const NODES: [&str; 4] = ["idref", "idrefList", "node", "nodeList"];

/// Each node's role and name, by its DOM node, for related nodes.
pub struct Related {
    by_backend: BTreeMap<i64, String>,
}

impl Related {
    /// From the tree's `nodes`.
    #[must_use]
    pub fn from_nodes(nodes: &[Value]) -> Self {
        let by_backend = nodes
            .iter()
            .filter_map(|n| {
                let role = n["role"]["value"].as_str().unwrap_or("?");
                let name = n["name"]["value"].as_str().unwrap_or("");
                Some((
                    n["backendDOMNodeId"].as_i64()?,
                    format!("{role} \"{name}\""),
                ))
            })
            .collect();
        Self { by_backend }
    }
}

/// `value` as the oracle writes it; `what` names it in a refusal.
///
/// # Errors
/// A value with no type or a type not read, or a related node not in the tree.
pub fn written(value: &Value, related: &Related, what: &str) -> Result<String, String> {
    let kind = value["type"]
        .as_str()
        .ok_or_else(|| format!("{what}: a value with no type: {value}"))?;
    if kind == "valueUndefined" {
        return Ok("undefined".to_owned());
    }
    if SCALARS.contains(&kind) {
        return Ok(value["value"]
            .as_str()
            .map_or_else(|| value["value"].to_string(), str::to_owned));
    }
    if NODES.contains(&kind) {
        return nodes(value, related, what);
    }
    Err(format!(
        "{what}: a value of type {kind} the oracle does not read: {value}"
    ))
}

fn nodes(value: &Value, related: &Related, what: &str) -> Result<String, String> {
    let list = value["relatedNodes"]
        .as_array()
        .ok_or_else(|| format!("{what}: no relatedNodes: {value}"))?;
    let written: Result<Vec<String>, String> = list
        .iter()
        .map(|n| {
            let id = n["backendDOMNodeId"].as_i64().unwrap_or(-1);
            related
                .by_backend
                .get(&id)
                .cloned()
                .ok_or_else(|| format!("{what}: a related node {id} is not in the a11y tree"))
        })
        .collect();
    Ok(written?.join(", "))
}

#[cfg(test)]
#[path = "ax_value_tests.rs"]
mod tests;
