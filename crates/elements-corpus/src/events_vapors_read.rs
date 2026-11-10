//! What the vapors player reads after each step: each named control's
//! value and checked state, every text field's selection, and the a11y
//! projection the script asks for.

use vapors_elements::Document;

use super::{Selection, find, name};

/// The document's a11y projection to `roles` as the oracle writes
/// Chromium's (`ax_projection`), or `None` without roles.
pub(super) fn projection(
    doc: &Document,
    roles: &[String],
) -> Result<Option<serde_json::Value>, String> {
    if roles.is_empty() {
        return Ok(None);
    }
    let roles: Vec<&str> = roles.iter().map(String::as_str).collect();
    let nodes = doc.ax_projection(&roles).map_err(|e| e.to_string())?;
    Ok(Some(serde_json::Value::Array(
        nodes.into_iter().map(ax_entry).collect(),
    )))
}

/// One node as the oracle writes Chromium's: its value only when it
/// has one.
fn ax_entry(node: vapors_elements::AxNode) -> serde_json::Value {
    let mut entry =
        serde_json::json!({ "role": node.role, "name": node.name, "properties": node.properties });
    if let Some(value) = node.value {
        entry["value"] = serde_json::json!(value);
    }
    entry
}

/// Each target's value: a target that names no element, or one that is
/// not a control with a value, is refused.
pub(super) fn values(doc: &Document, targets: &[String]) -> Result<Vec<(String, String)>, String> {
    targets
        .iter()
        .map(|target| {
            let node =
                find(doc, target).ok_or_else(|| format!("value {target} names no element"))?;
            let value = doc
                .value(node)
                .ok_or_else(|| format!("value {target} is not a control with a value"))?;
            Ok((target.clone(), value))
        })
        .collect()
}

/// Every text field's selection, by name, sorted by name.
pub(super) fn selections(doc: &Document) -> Vec<(String, Selection)> {
    let mut all: Vec<(String, Selection)> = doc
        .elements()
        .into_iter()
        .filter_map(|node| Some((name(doc, node), doc.selection(node)?)))
        .collect();
    all.sort();
    all
}

/// Each target's checked state: a target that names no element, or
/// one that is not a checkbox, is refused.
pub(super) fn checked(doc: &Document, targets: &[String]) -> Result<Vec<(String, bool)>, String> {
    targets
        .iter()
        .map(|target| {
            let node =
                find(doc, target).ok_or_else(|| format!("checked {target} names no element"))?;
            let checked = doc
                .checked(node)
                .ok_or_else(|| format!("checked {target} is not a checkbox"))?;
            Ok((target.clone(), checked))
        })
        .collect()
}
