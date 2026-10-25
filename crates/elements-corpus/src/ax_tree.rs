//! Chromium's accessibility tree (`Accessibility.getFullAXTree`), reduced
//! to what the a11y oracle commits: from the root, in order, each node
//! Chromium does not ignore, with its depth, role, name, value and
//! properties as `name=value`. An ignored node is left out and its
//! children kept at its depth. An `InlineTextBox` is left out: it is
//! how Chromium splits text for layout, not something a reader meets.
//! The page's `url` property is left out too ([`LEFT_OUT`]).

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

/// Properties left out: `url` names the temporary page the oracle wrote,
/// which differs from run to run.
const LEFT_OUT: [&str; 1] = ["url"];

/// The reduced tree as a list, from `getFullAXTree`'s result.
///
/// # Errors
/// A result without `nodes`, without a root, with a node without an id,
/// or naming a child that is not there.
pub fn reduce(result: &Value) -> Result<Value, String> {
    let nodes = result["nodes"]
        .as_array()
        .ok_or("the a11y tree has no `nodes`")?;
    let mut by_id = BTreeMap::new();
    for node in nodes {
        let id = node["nodeId"]
            .as_str()
            .ok_or_else(|| format!("an a11y node without a nodeId: {node}"))?;
        by_id.insert(id, node);
    }
    let root = nodes.first().ok_or("the a11y tree has no root")?;
    let mut out = Vec::new();
    walk(root, 0, &by_id, &mut out)?;
    Ok(Value::Array(out))
}

fn walk(
    node: &Value,
    depth: usize,
    by_id: &BTreeMap<&str, &Value>,
    out: &mut Vec<Value>,
) -> Result<(), String> {
    let role = node["role"]["value"].as_str().unwrap_or_default();
    let kept = node["ignored"] != true && role != "InlineTextBox";
    if kept {
        out.push(entry(node, depth));
    }
    let below = if kept { depth + 1 } else { depth };
    for child in node["childIds"].as_array().into_iter().flatten() {
        let id = child.as_str().unwrap_or_default();
        let child = by_id
            .get(id)
            .ok_or_else(|| format!("the a11y tree names a child {id} it does not hold"))?;
        walk(child, below, by_id, out)?;
    }
    Ok(())
}

/// One node: depth and role always; name and value when not empty;
/// properties when there are any, sorted.
fn entry(node: &Value, depth: usize) -> Value {
    let mut entry = Map::new();
    entry.insert("depth".into(), json!(depth));
    entry.insert("role".into(), node["role"]["value"].clone());
    for key in ["name", "value"] {
        let text = node[key]["value"].as_str().unwrap_or_default();
        if !text.is_empty() {
            entry.insert(key.into(), json!(text));
        }
    }
    let mut properties: Vec<String> = node["properties"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|p| !LEFT_OUT.contains(&p["name"].as_str().unwrap_or_default()))
        .map(|p| {
            format!(
                "{}={}",
                p["name"].as_str().unwrap_or("?"),
                bare(&p["value"]["value"])
            )
        })
        .collect();
    properties.sort();
    if !properties.is_empty() {
        entry.insert("properties".into(), json!(properties));
    }
    Value::Object(entry)
}

/// A property's value as written: a string without quotes.
fn bare(value: &Value) -> String {
    value
        .as_str()
        .map_or_else(|| value.to_string(), str::to_owned)
}

#[cfg(test)]
#[path = "ax_tree_tests.rs"]
mod tests;
