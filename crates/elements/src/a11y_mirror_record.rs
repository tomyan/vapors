//! One node as the mirror's record (R6): what `ax_tree` says of it
//! (role, name, value, description, states, bounds), its range and
//! focusability, and its parent in the tree: the nearest node shown
//! above it, carried down a document-order pass ([`AboveMemo`]); a node
//! under an `aria-hidden` ancestor is not in the tree. Read from the last update,
//! as `ax_tree` is.

use vapors_dom::NodeId;

use crate::a11y_error::AxError;
use crate::a11y_mirror::{AxMirrorNode, AxRange};
use crate::a11y_mirror_roles::unexposable;
use crate::a11y_mirror_state::AxIndex;
use crate::a11y_tree::finite;
use crate::document::Document;
use crate::layout::BoxRect;
use crate::node_map::NodeMap;

/// What is above a node: an `aria-hidden` ancestor, or its parent.
#[derive(Clone, Copy)]
pub(crate) enum Above {
    Hidden,
    Under(Option<NodeId>),
}

/// What is above each element, filled in document order so that every
/// element's entry is its parent's entry and its parent's own record:
/// one lookup per element, never a walk.
#[derive(Default)]
pub(crate) struct AboveMemo(NodeMap<Above>);

impl AboveMemo {
    /// What is above `node`, from its parent's entry; the root is under
    /// nothing.
    pub(crate) fn over(&self, doc: &Document, node: NodeId) -> Above {
        doc.parent(node)
            .map_or(Above::Under(None), |parent| self.0[&parent])
    }

    /// Notes what `node`'s children are under: hidden when `node` or what
    /// is above it hides them, `node` when it is shown, else what is
    /// above it.
    pub(crate) fn note(&mut self, doc: &Document, node: NodeId, over: Above, shown: bool) {
        let entry = match over {
            _ if doc.attribute(node, "aria-hidden") == Some("true") => Above::Hidden,
            Above::Hidden => Above::Hidden,
            Above::Under(_) if shown => Above::Under(Some(node)),
            over @ Above::Under(_) => over,
        };
        self.0.insert(node, entry);
    }
}

impl Document {
    /// `node` as a record, given what is above it.
    pub(crate) fn ax_record_under(
        &self,
        node: NodeId,
        index: &AxIndex,
        above: Above,
    ) -> Result<Option<AxMirrorNode>, AxError> {
        let Above::Under(parent) = above else {
            return Ok(None);
        };
        let Some((bounds, role)) = self.ax_in_tree(node, index) else {
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
        match unexposable(self, &record) {
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
