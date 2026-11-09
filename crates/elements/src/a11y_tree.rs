//! The a11y tree a driver reads (R1): every node with a role
//! (`a11y_roles`) that the last update drew, nested under its nearest
//! ancestor in the tree, in document order. A node is left out when it
//! has no box (`display: none` on it or an ancestor) or is
//! `visibility: hidden`; it and its subtree when it is
//! `aria-hidden="true"`. The tree is read from the last update, so a
//! document changed since then is refused rather than answered from it.

use vapors_dom::NodeId;

use crate::a11y_error::AxError;
use crate::a11y_roles::Ids;
use crate::document::Document;
use crate::layout::BoxRect;
use crate::painted::Painted;

/// One node of the tree.
#[derive(Clone, Debug, PartialEq)]
pub struct AxElement {
    /// The element.
    pub node: NodeId,
    /// Its role.
    pub role: String,
    /// Its accessible name; empty for none.
    pub name: String,
    /// Its value, for a control that shows one, when not empty.
    pub value: Option<String>,
    /// Its states.
    pub states: AxStates,
    /// Its border box in CSS pixels, as the last update placed it.
    pub bounds: BoxRect,
    /// The nodes nearest under it in the tree, in document order.
    pub children: Vec<AxElement>,
}

/// A node's states; `None` where Chromium leaves the state out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AxStates {
    /// It has focus.
    pub focused: bool,
    /// A disabled control, or `aria-disabled="true"`.
    pub disabled: bool,
    /// A checkbox's state, or `aria-checked`.
    pub checked: Option<bool>,
    /// `aria-expanded`.
    pub expanded: Option<bool>,
    /// `aria-selected="true"`.
    pub selected: bool,
    /// `aria-pressed`.
    pub pressed: Option<bool>,
}

impl AxStates {
    /// The states Chromium writes, as `name=value`, sorted.
    #[must_use]
    pub fn set(&self) -> Vec<String> {
        let mut set = Vec::new();
        for (name, state) in [
            ("checked", self.checked),
            ("expanded", self.expanded),
            ("pressed", self.pressed),
        ] {
            if let Some(state) = state {
                set.push(format!("{name}={state}"));
            }
        }
        for (name, state) in [
            ("disabled", self.disabled),
            ("focused", self.focused),
            ("selected", self.selected),
        ] {
            if state {
                set.push(format!("{name}=true"));
            }
        }
        set.sort();
        set
    }
}

impl Document {
    /// The a11y tree's top nodes, each with its subtree.
    ///
    /// # Errors
    /// [`AxError::NotUpdated`]: the document has changed since its last
    /// update, or has none.
    pub fn ax_tree(&self) -> Result<Vec<AxElement>, AxError> {
        let painted = self.painted.as_ref().filter(|_| !self.changes.since_update);
        let painted = painted.ok_or(AxError::NotUpdated)?;
        let ids = self.ax_ids();
        Ok(self.ax_under(self.root(), painted, &ids))
    }

    /// The tree's nodes under `node`, which is not one of them.
    fn ax_under(&self, node: NodeId, painted: &Painted, ids: &Ids) -> Vec<AxElement> {
        let mut out = Vec::new();
        for child in self.children(node) {
            if self.tag(child).is_none() || self.attribute(child, "aria-hidden") == Some("true") {
                continue;
            }
            match self.ax_element(child, painted, ids) {
                Some(element) => out.push(element),
                None => out.extend(self.ax_under(child, painted, ids)),
            }
        }
        out
    }

    /// `node` as a node of the tree, when it has a role and was drawn.
    fn ax_element(&self, node: NodeId, painted: &Painted, ids: &Ids) -> Option<AxElement> {
        let bounds = painted
            .border_box(node)
            .filter(|_| painted.is_visible(node))?;
        let role = self.ax_role(node, ids)?;
        Some(AxElement {
            node,
            name: self.ax_name(node, &role, ids),
            value: self.ax_value(node, &role),
            states: self.ax_states(node),
            bounds,
            children: self.ax_under(node, painted, ids),
            role,
        })
    }
}
