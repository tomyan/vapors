//! What one node is to the a11y tree, as Chromium's accessibility tree
//! has it (pinned by the corpus's `roles.events` and `combobox.events`):
//! its role, accessible name, value and states.
//!
//! Roles are `role`'s first valid WAI-ARIA token (`none`,
//! `presentation` and `generic` are no role), else implicit: `button`;
//! `input` as `textbox`, `checkbox`, `slider` (range) or `spinbutton`
//! (number); a `section` with a name as `region`. Names: `a11y_name`.
//! Values are a slider's, spinbutton's, textbox's or combobox's `value`
//! when not empty, else a slider's or spinbutton's `aria-valuenow`
//! (Chromium's value; `aria-valuetext` is not it). States are written as Chromium
//! writes them (`AxStates`).

use std::collections::HashMap;

use vapors_dom::NodeId;

use crate::a11y_tree::AxStates;
use crate::document::Document;

/// Each element id, to its first element in document order.
pub(crate) type Ids = HashMap<String, NodeId>;

/// WAI-ARIA 1.2's roles, abstract ones left out: `role`'s first of
/// these is the node's role, as Chromium takes it; other tokens are
/// skipped.
const ARIA_ROLES: [&str; 81] = [
    "alert",
    "alertdialog",
    "application",
    "article",
    "banner",
    "blockquote",
    "button",
    "caption",
    "cell",
    "checkbox",
    "code",
    "columnheader",
    "combobox",
    "complementary",
    "contentinfo",
    "definition",
    "deletion",
    "dialog",
    "document",
    "emphasis",
    "feed",
    "figure",
    "form",
    "generic",
    "grid",
    "gridcell",
    "group",
    "heading",
    "img",
    "insertion",
    "link",
    "list",
    "listbox",
    "listitem",
    "log",
    "main",
    "marquee",
    "math",
    "menu",
    "menubar",
    "menuitem",
    "menuitemcheckbox",
    "menuitemradio",
    "meter",
    "navigation",
    "none",
    "note",
    "option",
    "paragraph",
    "presentation",
    "progressbar",
    "radio",
    "radiogroup",
    "region",
    "row",
    "rowgroup",
    "rowheader",
    "scrollbar",
    "search",
    "searchbox",
    "separator",
    "slider",
    "spinbutton",
    "status",
    "strong",
    "subscript",
    "superscript",
    "switch",
    "tab",
    "table",
    "tablist",
    "tabpanel",
    "term",
    "textbox",
    "time",
    "timer",
    "toolbar",
    "tooltip",
    "tree",
    "treegrid",
    "treeitem",
];

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
            .and_then(|r| r.split_whitespace().find(|t| ARIA_ROLES.contains(t)))
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

    /// `node`'s value, when its role shows one and it is not empty.
    pub(crate) fn ax_value(&self, node: NodeId, role: &str) -> Option<String> {
        if !VALUED.contains(&role) {
            return None;
        }
        let ranged = role == "slider" || role == "spinbutton";
        let now = || self.attribute(node, "aria-valuenow").map(str::to_owned);
        self.value(node)
            .or_else(|| now().filter(|_| ranged))
            .filter(|value| !value.is_empty())
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
}
