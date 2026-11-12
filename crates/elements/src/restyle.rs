//! Restyle (F2c S7): each element's computed and used style is kept
//! between updates, and an update restyles only what the changes since
//! the last one marked (`restyle_pass`). A sheet added, a viewport other
//! than the last, or a failed restyle restyles everything.

use crate::node_map::{NodeMap, NodeSet};

use vapors_css::cascade::ComputedStyle;
use vapors_css::tree::States;
use vapors_dom::NodeId;

use crate::document::Document;
use crate::error::ElementError;
use crate::style::{Styles, Used};
use crate::update::Viewport;

/// What changed since the last update.
#[derive(Default)]
pub(crate) struct Dirty {
    pub(crate) all: bool,
    /// Subtrees to restyle whole.
    pub(crate) roots: Vec<NodeId>,
    /// Elements to restyle, and their children's subtrees only if their
    /// own style changes.
    pub(crate) elements: Vec<NodeId>,
    /// A child was inserted or removed and its siblings marked by what
    /// the sheets reach (`restyle_children`): debug builds check the
    /// restyle against a fresh one (`restyle_check`).
    pub(crate) narrowed: bool,
}

impl Dirty {
    /// Everything: a sheet was added.
    pub(crate) fn everything(&mut self) {
        self.all = true;
    }

    /// `node` and its descendants.
    pub(crate) fn subtree(&mut self, node: NodeId) {
        self.roots.push(node);
    }

    /// `node`, and its descendants only if its style changes.
    pub(crate) fn element(&mut self, node: NodeId) {
        self.elements.push(node);
    }
}

/// The styles kept from the last update.
pub(crate) struct Kept {
    pub(crate) viewport: Viewport,
    pub(crate) initial: ComputedStyle,
    pub(crate) styles: Styles,
    pub(crate) used: NodeMap<Used>,
    /// Each element's refusals, kept until it is restyled again.
    pub(crate) refused: NodeMap<Vec<ElementError>>,
    /// Which elements the last update restyled: their computed styles,
    /// and so the addresses their calc handles hold, are new.
    pub(crate) restyled_nodes: NodeSet,
    /// The calcs of the kept styles, by the handles boxes hold.
    pub(crate) calcs: crate::layout::Calcs,
}

impl Kept {
    /// Whether `node`'s box may take the last update's layout: not when
    /// it was restyled and its style holds a calc handle, which compares
    /// by address (see `layout::keep`).
    pub(crate) fn may_keep_layout(&self, node: NodeId) -> bool {
        !self.restyled_nodes.contains(&node)
            || !self.styles.get(node).is_some_and(|style| {
                style
                    .iter()
                    .any(|(_, value)| matches!(value, vapors_css::value::Value::Calc(_)))
            })
    }
}

impl Document {
    /// Marks what a change to `node`'s attributes restyles: its
    /// subtree, its parent's when a sibling's style can depend on it,
    /// everything when `:has()` is used.
    pub(crate) fn mark_changed(&mut self, node: NodeId) {
        let features = self.sheets.features;
        if features.has {
            self.dirty.everything();
        } else if features.siblings.any() {
            self.dirty.subtree(self.parent(node).unwrap_or(node));
        } else {
            self.dirty.subtree(node);
        }
    }

    /// Marks what `flag` flipping on `node` restyles, as the sheets'
    /// selectors use it (`Cascade::restyle_scope_of_selectors`): a flag
    /// only on a subject compound restyles the element alone, and its
    /// children only if its own style changes, which covers what they
    /// inherit.
    pub(crate) fn mark_flag(&mut self, node: NodeId, flag: States) {
        let scope = self.sheets.restyle_scope(flag);
        if scope.whole_tree {
            self.dirty.everything();
        } else if scope.later_siblings {
            self.dirty.subtree(self.parent(node).unwrap_or(node));
        } else if scope.descendants {
            self.dirty.subtree(node);
        } else if scope.element {
            self.dirty.element(node);
        }
    }
}

impl Document {
    /// The computed style the last update kept for `node`, if it is an
    /// element in the document.
    #[must_use]
    pub fn computed_style(&self, node: NodeId) -> Option<&ComputedStyle> {
        self.kept.as_ref()?.styles.get(node)
    }

    /// Drops every style, layout, text layout and paint kept between
    /// updates, so the next update works from scratch: for tests that compare an
    /// incremental update with a full one.
    #[doc(hidden)]
    pub fn forget_kept(&mut self) {
        self.kept = None;
        self.checks = crate::kept_checks::KeptChecks::default();
        self.boxes = crate::layout::KeptBoxes::default();
        self.texts = crate::layout::TextCache::default();
        self.painted = None;
        self.dirty.everything();
    }
}
