//! Chromium's accessibility tree (`Accessibility.getFullAXTree`), reduced
//! to what the a11y oracle commits: from the root, in order, each node
//! Chromium does not ignore, with its depth, role, and its name, value and
//! description when not empty, and its properties as sorted `name=value`
//! (`ax_value` writes each value). An ignored node is left out and its
//! children kept at its depth. An `InlineTextBox` is left out: it is how
//! Chromium splits text for layout, not something a reader meets. The
//! page's `url` property is left out ([`LEFT_OUT`]).
//!
//! The structure is checked: node ids are unique, every node has a role,
//! no node is reached twice, and every node is under the root. A missing
//! `childIds` is a leaf: the protocol makes it optional.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value, json};

use crate::ax_value::{Related, written};

/// Properties left out: `url` names the temporary page the oracle wrote,
/// which differs from run to run.
const LEFT_OUT: [&str; 1] = ["url"];

/// A walk over the tree: the nodes by id, the related names, what has
/// been reached and what has been written.
struct Walk<'a> {
    by_id: BTreeMap<&'a str, &'a Value>,
    related: Related,
    reached: BTreeSet<&'a str>,
    out: Vec<Value>,
}

/// The reduced tree as a list, from `getFullAXTree`'s result.
///
/// # Errors
/// A result without `nodes` or a root, a node without an id or a role, a
/// duplicate id, a node reached twice or not at all, a child that is not
/// there, or a value `ax_value` refuses.
pub fn reduce(result: &Value) -> Result<Value, String> {
    let nodes = result["nodes"]
        .as_array()
        .ok_or("the a11y tree has no `nodes`")?;
    let root = nodes.first().ok_or("the a11y tree has no root")?;
    let mut walk = Walk {
        by_id: index(nodes)?,
        related: Related::from_nodes(nodes),
        reached: BTreeSet::new(),
        out: Vec::new(),
    };
    walk.node(root, 0)?;
    let missed = nodes.len() - walk.reached.len();
    if missed > 0 {
        return Err(format!(
            "{missed} a11y node{} not under the root",
            if missed == 1 { " is" } else { "s are" }
        ));
    }
    Ok(Value::Array(walk.out))
}

fn index(nodes: &[Value]) -> Result<BTreeMap<&str, &Value>, String> {
    let mut by_id = BTreeMap::new();
    for node in nodes {
        let id = node["nodeId"]
            .as_str()
            .ok_or_else(|| format!("an a11y node without a nodeId: {node}"))?;
        if by_id.insert(id, node).is_some() {
            return Err(format!("two a11y nodes with id {id}"));
        }
    }
    Ok(by_id)
}

impl<'a> Walk<'a> {
    fn node(&mut self, node: &'a Value, depth: usize) -> Result<(), String> {
        let id = node["nodeId"].as_str().unwrap_or("?");
        if !self.reached.insert(id) {
            return Err(format!(
                "a11y node {id} is reached twice: the tree has a cycle"
            ));
        }
        let role = node["role"]["value"]
            .as_str()
            .ok_or_else(|| format!("a11y node {id} has no role"))?;
        let kept = node["ignored"] != true && role != "InlineTextBox";
        if kept {
            let entry = entry(node, depth, role, &self.related)?;
            self.out.push(entry);
        }
        self.children(node, if kept { depth + 1 } else { depth })?;
        Ok(())
    }

    fn children(&mut self, node: &'a Value, depth: usize) -> Result<(), String> {
        for child in node["childIds"].as_array().into_iter().flatten() {
            let id = child.as_str().unwrap_or_default();
            let child = *self
                .by_id
                .get(id)
                .ok_or_else(|| format!("the a11y tree names a child {id} it does not hold"))?;
            self.node(child, depth)?;
        }
        Ok(())
    }
}

/// One node: depth and role; name, value and description when not
/// empty; properties when there are any.
fn entry(node: &Value, depth: usize, role: &str, related: &Related) -> Result<Value, String> {
    let mut entry = Map::new();
    entry.insert("depth".into(), json!(depth));
    entry.insert("role".into(), json!(role));
    for key in ["name", "value", "description"] {
        if node.get(key).is_none() {
            continue;
        }
        let text = written(&node[key], related, key)?;
        if !text.is_empty() {
            entry.insert(key.into(), json!(text));
        }
    }
    let properties = properties(node, related)?;
    if !properties.is_empty() {
        entry.insert("properties".into(), json!(properties));
    }
    Ok(Value::Object(entry))
}

/// The node's properties as sorted `name=value`, [`LEFT_OUT`] aside.
fn properties(node: &Value, related: &Related) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for property in node["properties"].as_array().into_iter().flatten() {
        let name = property["name"]
            .as_str()
            .ok_or_else(|| format!("an a11y property without a name: {property}"))?;
        if !LEFT_OUT.contains(&name) {
            out.push(format!(
                "{name}={}",
                written(&property["value"], related, name)?
            ));
        }
    }
    out.sort();
    Ok(out)
}

#[cfg(test)]
#[path = "ax_tree_tests.rs"]
mod tests;
