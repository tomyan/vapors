//! What one node is to the a11y tree, as Chromium's accessibility tree
//! has it (pinned by the corpus's `roles.events` and `combobox.events`):
//! its role, accessible name, value and states.
//!
//! Roles are `role`'s first token (`none`, `presentation` and
//! `generic` are no role), else implicit: `button`; `input` as
//! `textbox`, `checkbox`, `slider` (range) or `spinbutton` (number); a
//! `section` with a name as `region`. Names are `aria-label`,
//! `aria-labelledby`, or the content of a button, option or checkbox.
//! `label` arrives in F4. Values are a slider's, spinbutton's, textbox's
//! or combobox's `value` when not empty. States are written as Chromium
//! writes them (`AxStates`).

use std::collections::HashMap;

use vapors_dom::NodeId;

use crate::a11y_tree::AxStates;
use crate::document::Document;

/// Each element id, to its first element in document order.
pub(crate) type Ids = HashMap<String, NodeId>;

/// Roles whose name is their content.
const NAMED_BY_CONTENT: [&str; 3] = ["option", "button", "checkbox"];

/// Role tokens that give no role.
const NO_ROLE: [&str; 3] = ["none", "presentation", "generic"];

/// Roles that show a value.
const VALUED: [&str; 4] = ["slider", "spinbutton", "textbox", "combobox"];

impl Document {
    /// Each id once, so a lookup is not a scan: the first in document
    /// order wins, as `getElementById`.
    pub(crate) fn ax_ids(&self) -> Ids {
        let mut ids = HashMap::new();
        for node in self.elements() {
            if let Some(id) = self.attribute(node, "id") {
                ids.entry(id.to_owned()).or_insert(node);
            }
        }
        ids
    }

    /// `node`'s role: its `role` attribute's first token, else implicit.
    pub(crate) fn ax_role(&self, node: NodeId, ids: &Ids) -> Option<String> {
        if let Some(role) = self
            .attribute(node, "role")
            .and_then(|r| r.split_whitespace().next())
        {
            return (!NO_ROLE.contains(&role)).then(|| role.to_owned());
        }
        let implicit = match (self.tag(node)?, self.attribute(node, "type")) {
            ("input", _) if self.is_text_input(node) => "textbox",
            ("input", Some("checkbox")) => "checkbox",
            ("input", Some("range")) => "slider",
            ("input", Some("number")) => "spinbutton",
            ("button", _) => "button",
            ("section", _) if !self.ax_name(node, "region", ids).is_empty() => "region",
            _ => return None,
        };
        Some(implicit.to_owned())
    }

    /// `node`'s accessible name, empty for none.
    pub(crate) fn ax_name(&self, node: NodeId, role: &str, ids: &Ids) -> String {
        if let Some(label) = self.attribute(node, "aria-label") {
            return label.trim().to_owned();
        }
        if let Some(labels) = self.attribute(node, "aria-labelledby") {
            let parts: Vec<String> = labels
                .split_whitespace()
                .filter_map(|id| ids.get(id).copied())
                .map(|n| self.content(n))
                .collect();
            return parts.join(" ");
        }
        if NAMED_BY_CONTENT.contains(&role) {
            return self.content(node);
        }
        String::new()
    }

    /// `node`'s value, when its role shows one and it is not empty.
    pub(crate) fn ax_value(&self, node: NodeId, role: &str) -> Option<String> {
        if !VALUED.contains(&role) {
            return None;
        }
        self.value(node).filter(|value| !value.is_empty())
    }

    /// `node`'s states.
    pub(crate) fn ax_states(&self, node: NodeId) -> AxStates {
        let flag = |name| match self.attribute(node, name) {
            Some("true") => Some(true),
            Some("false") => Some(false),
            _ => None,
        };
        AxStates {
            focused: self.active_element() == Some(node),
            disabled: self.is_disabled_control(node) || flag("aria-disabled") == Some(true),
            checked: self.checked(node).or_else(|| flag("aria-checked")),
            expanded: flag("aria-expanded"),
            selected: flag("aria-selected") == Some(true),
            pressed: flag("aria-pressed"),
        }
    }

    /// `node`'s text content, whitespace collapsed.
    fn content(&self, node: NodeId) -> String {
        let mut text = String::new();
        self.collect_text(node, &mut text);
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn collect_text(&self, node: NodeId, out: &mut String) {
        if let Some(own) = self.text(node) {
            out.push_str(own);
            out.push(' ');
        }
        for child in self.children(node) {
            self.collect_text(child, out);
        }
    }
}
