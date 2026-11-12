//! One node as the mirror's record (R6): what `ax_tree` says of it
//! (role, name, value, description, states, bounds), its range and
//! focusability, and its parent in the tree, found by walking up to the
//! nearest node shown; a node under an `aria-hidden` ancestor is not
//! in the tree. Read from the last update, as `ax_tree` is.

use vapors_dom::NodeId;

use crate::a11y_error::AxError;
use crate::a11y_mirror::{AxMirrorNode, AxRange};
use crate::a11y_mirror_roles::unexposable;
use crate::a11y_mirror_state::AxIndex;
use crate::a11y_tree::finite;
use crate::document::Document;
use crate::layout::BoxRect;

/// What is above a node: an `aria-hidden` ancestor, or its parent.
enum Above {
    Hidden,
    Under(Option<NodeId>),
}

impl Document {
    /// `node` as a record, when it is in the tree now.
    ///
    /// # Errors
    /// [`AxError::Unexposable`]; [`AxError::NotFinite`].
    pub(crate) fn ax_record(
        &self,
        node: NodeId,
        index: &AxIndex,
    ) -> Result<Option<AxMirrorNode>, AxError> {
        let Some((bounds, role)) = self.ax_in_tree(node, index) else {
            return Ok(None);
        };
        let Above::Under(parent) = self.ax_above(node, index) else {
            return Ok(None);
        };
        let (name, description) = self.ax_name_and_description(node, &role, &index.ids);
        finite(bounds, &role, &name)?;
        let record = AxMirrorNode {
            node,
            parent,
            value: self.ax_value(node, &role),
            description,
            states: self.ax_states(node),
            range: self.ax_range(node, &role),
            focusable: self.focus_candidate(node),
            bounds,
            name,
            role,
        };
        match unexposable(&record) {
            Some(needs) => Err(AxError::Unexposable {
                role: record.role,
                name: record.name,
                needs,
            }),
            None => Ok(Some(record)),
        }
    }

    /// `node`'s box and role when it is an element in the document that
    /// the last update drew, visible, with a role and not `aria-hidden`.
    fn ax_in_tree(&self, node: NodeId, index: &AxIndex) -> Option<(BoxRect, String)> {
        if !self.contains(node) || self.tag(node).is_none() {
            return None;
        }
        if self.attribute(node, "aria-hidden") == Some("true") {
            return None;
        }
        let painted = self.painted.as_ref()?;
        let bounds = painted
            .border_box(node)
            .filter(|_| painted.is_visible(node))?;
        let role = self.ax_role(node, &index.ids)?;
        Some((bounds, role))
    }

    /// The nearest shown ancestor of `node`, unless an ancestor hides it.
    fn ax_above(&self, node: NodeId, index: &AxIndex) -> Above {
        let mut parent = None;
        let mut cursor = self.parent(node);
        while let Some(ancestor) = cursor {
            if self.attribute(ancestor, "aria-hidden") == Some("true") {
                return Above::Hidden;
            }
            if parent.is_none() && self.ax_in_tree(ancestor, index).is_some() {
                parent = Some(ancestor);
            }
            cursor = self.parent(ancestor);
        }
        Above::Under(parent)
    }

    /// A slider's or spinbutton's range: the element's, else its
    /// `aria-valuemin` and `aria-valuemax`.
    fn ax_range(&self, node: NodeId, role: &str) -> Option<AxRange> {
        if role != "slider" && role != "spinbutton" {
            return None;
        }
        if let Some(range) = self.range(node) {
            return Some(AxRange {
                min: range.min,
                max: range.max,
            });
        }
        let number = |name: &str| self.attribute(node, name)?.parse::<f64>().ok();
        Some(AxRange {
            min: number("aria-valuemin")?,
            max: number("aria-valuemax")?,
        })
    }
}
