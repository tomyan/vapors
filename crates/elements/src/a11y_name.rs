//! A node's accessible name, as Chromium computes it (pinned by the
//! corpus's `names.events` and `roles.events`): a non-empty
//! `aria-label`; else the content of the `aria-labelledby` ids found,
//! when not empty; else, for a role named by its content, the text it
//! renders. `label` arrives in F4.
//!
//! Content is the text of the subtree as rendered: an element with
//! `display: none` or `aria-hidden="true"` adds nothing, text under
//! `visibility: hidden` adds nothing, inline elements join their
//! neighbours' text and other boxes are set apart by a space, as
//! Chromium's name from content is; a named icon (`<svg role="img">`)
//! adds its `aria-label`. Whitespace is collapsed and trimmed.

use vapors_dom::NodeId;

use crate::a11y_roles::Ids;
use crate::document::Document;

/// Roles whose name is their content.
const NAMED_BY_CONTENT: [&str; 3] = ["option", "button", "checkbox"];

impl Document {
    /// `node`'s accessible name, empty for none.
    pub(crate) fn ax_name(&self, node: NodeId, role: &str, ids: &Ids) -> String {
        let label = self.attribute(node, "aria-label").map(str::trim);
        if let Some(label) = label.filter(|label| !label.is_empty()) {
            return label.to_owned();
        }
        let labelled = self.ax_labelled_by(node, ids);
        if !labelled.is_empty() || !NAMED_BY_CONTENT.contains(&role) {
            return labelled;
        }
        self.ax_content(node)
    }

    /// The content of each `aria-labelledby` id found, space separated.
    fn ax_labelled_by(&self, node: NodeId, ids: &Ids) -> String {
        let Some(labels) = self.attribute(node, "aria-labelledby") else {
            return String::new();
        };
        let parts: Vec<String> = labels
            .split_whitespace()
            .filter_map(|id| ids.get(id).copied())
            .map(|label| self.ax_content(label))
            .filter(|part| !part.is_empty())
            .collect();
        parts.join(" ")
    }

    /// The text `node`'s subtree renders, whitespace collapsed.
    fn ax_content(&self, node: NodeId) -> String {
        let mut text = String::new();
        for child in self.children(node) {
            self.collect_rendered(child, self.ax_shown(node), &mut text);
        }
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// `node`'s rendered text onto `out`; `shown` is whether its parent
    /// is `visibility: visible`.
    fn collect_rendered(&self, node: NodeId, shown: bool, out: &mut String) {
        if let Some(own) = self.text(node) {
            if shown {
                out.push_str(own);
            }
            return;
        }
        let Some(inline) = self.ax_rendered_inline(node) else {
            return;
        };
        let apart = if inline { "" } else { " " };
        out.push_str(apart);
        if let Some(label) = self.ax_icon_label(node) {
            out.push_str(label);
            out.push_str(apart);
            return;
        }
        for child in self.children(node) {
            self.collect_rendered(child, self.ax_shown(node), out);
        }
        out.push_str(apart);
    }

    /// A named icon's `aria-label` (`<svg role="img">`, docs/icons.md),
    /// which a name from content takes in place of its paths.
    fn ax_icon_label(&self, node: NodeId) -> Option<&str> {
        let label = self.attribute(node, "aria-label").map(str::trim)?;
        let named = self.tag(node) == Some("svg") && self.attribute(node, "role") == Some("img");
        (named && !label.is_empty()).then_some(label)
    }

    /// Whether element `node` is inline; `None` when it renders nothing
    /// (`display: none`, `aria-hidden`, or not styled by the last update).
    fn ax_rendered_inline(&self, node: NodeId) -> Option<bool> {
        if self.attribute(node, "aria-hidden") == Some("true") {
            return None;
        }
        let used = self.kept.as_ref()?.used.get(&node)?;
        (used.layout.display != taffy::Display::None).then_some(used.inline)
    }

    /// Whether `node` is `visibility: visible` in the last update.
    fn ax_shown(&self, node: NodeId) -> bool {
        let used = self.kept.as_ref().and_then(|kept| kept.used.get(&node));
        used.is_some_and(|used| used.visible)
    }
}
