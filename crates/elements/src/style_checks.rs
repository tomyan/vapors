//! The style phase's checks, redone only for the elements changed since
//! the last update (`kept_checks`). When the kept checks say an element
//! refuses, a walk over the whole tree collects the refusals in
//! document order, and it must find one. Debug builds walk on every
//! update and assert the kept checks agree, so a missed mark fails the
//! tests that miss it.

use std::collections::HashSet;

use vapors_dom::NodeId;

use crate::check::{check_element, check_tree, describe, elements, style_attribute_refusal};
use crate::document::Document;
use crate::error::ElementError;
use crate::kept_checks::KeptChecks;

impl Document {
    /// The elements under the root to check again: every one when
    /// nothing was checked yet or every box is marked; else the marked
    /// elements and each added subtree.
    pub(crate) fn touched(&self) -> Vec<NodeId> {
        if self.checks.all || self.boxes.marks.all {
            return elements(&self.arena, self.root());
        }
        let marked = self
            .boxes
            .marks
            .nodes()
            .chain(self.checks.marked.iter().copied());
        let mut nodes: Vec<NodeId> = marked.filter(|&node| self.attached(node)).collect();
        for &added in self
            .checks
            .added
            .iter()
            .filter(|&&node| self.attached(node))
        {
            nodes.extend(elements(&self.arena, added));
        }
        nodes
    }

    /// `touched` and the elements restyled, each once.
    pub(crate) fn rechecked(&self, touched: Vec<NodeId>) -> Vec<NodeId> {
        let restyled = &self
            .kept
            .as_ref()
            .expect("restyle keeps the styles")
            .restyled_nodes;
        let all: HashSet<NodeId> = touched
            .into_iter()
            .chain(restyled.iter().copied())
            .collect();
        let mut nodes: Vec<NodeId> = all
            .into_iter()
            .filter(|&node| self.attached(node))
            .collect();
        nodes.sort_by_key(|node| node.0);
        nodes
    }

    /// Whether `node` is in the tree under the root.
    pub(crate) fn attached(&self, node: NodeId) -> bool {
        self.arena.contains(node) && self.chain(node).last() == Some(&self.root())
    }

    /// Checks `nodes`' tags, attributes, text, listeners and `style`
    /// attributes again.
    pub(crate) fn recheck_tree(&mut self, nodes: &[NodeId]) {
        for &node in nodes {
            let refuses = !check_element(&self.arena, node, &self.declared_events).is_empty();
            KeptChecks::note(&mut self.checks.tree, node, refuses);
            let refuses = style_attribute_refusal(&self.arena, node).is_some();
            KeptChecks::note(&mut self.checks.style_attribute, node, refuses);
        }
    }

    /// Checks `nodes`' used styles and media again and keeps their calcs.
    pub(crate) fn recheck_styled(&mut self, nodes: &[NodeId], scale: f32) {
        for &node in nodes {
            self.counts.elements_visited += 1;
            let refuses = !self.styled_refusals(&[node], scale).is_empty();
            KeptChecks::note(&mut self.checks.styled, node, refuses);
            let kept = self.kept.as_mut().expect("restyle keeps the styles");
            let arena = &self.arena;
            kept.calcs
                .set(node, kept.styles.of(node), || describe(arena, node));
        }
        #[cfg(debug_assertions)]
        self.assert_calcs_kept();
    }

    /// The refusals after styling of `nodes`: each one's used style and,
    /// for an inline element, mixed metrics; then their media's.
    fn styled_refusals(&self, nodes: &[NodeId], scale: f32) -> Vec<ElementError> {
        let kept = self.kept.as_ref().expect("restyle keeps the styles");
        let mut errors = Vec::new();
        for &node in nodes {
            errors.extend(kept.refused.get(&node).into_iter().flatten().cloned());
            if kept.used.get(&node).is_some_and(|style| style.inline) {
                errors.extend(self.mixed_metrics(node, &kept.used, scale));
            }
        }
        errors.extend(crate::media_checks::refusals(
            self, nodes, &kept.used, scale,
        ));
        errors
    }

    /// The tree's refusals, in document order.
    pub(crate) fn tree_refusals(&self) -> Vec<ElementError> {
        let walk = || check_tree(&self.arena, self.root(), &self.declared_events);
        walked(!self.checks.tree.is_empty(), walk, "tree")
    }

    /// The `style` attributes that do not parse, in document order.
    pub(crate) fn style_attribute_refusals(&self) -> Vec<ElementError> {
        let walk = || crate::check::style_attribute_refusals(&self.arena, self.root());
        walked(
            !self.checks.style_attribute.is_empty(),
            walk,
            "style attribute",
        )
    }

    /// The refusals after styling, in document order.
    pub(crate) fn all_styled_refusals(&self, scale: f32) -> Vec<ElementError> {
        let walk = || self.styled_refusals(&elements(&self.arena, self.root()), scale);
        walked(!self.checks.styled.is_empty(), walk, "styled")
    }

    /// Drops what is kept for `node` and its subtree, which leave the
    /// tree.
    pub(crate) fn forget_subtree(&mut self, node: NodeId) {
        for node in elements(&self.arena, node) {
            self.checks.forget(node);
            if let Some(kept) = self.kept.as_mut() {
                kept.used.remove(&node);
                kept.refused.remove(&node);
                kept.calcs.forget(node);
            }
        }
    }

    #[cfg(debug_assertions)]
    fn assert_calcs_kept(&self) {
        let kept = self.kept.as_ref().expect("restyle keeps the styles");
        let mut fresh = crate::layout::Calcs::default();
        for node in elements(&self.arena, self.root()) {
            fresh.set(node, kept.styles.of(node), String::new);
        }
        assert_eq!(
            kept.calcs.handles(),
            fresh.handles(),
            "the kept calcs differ from the calcs of the kept styles: an element restyled or removed was not checked again (docs/plans/iu3.md, PR 3a); reproduce with the failing test"
        );
    }
}

/// The refusals a walk over the tree finds, when the kept checks say
/// some element refuses, or always in a debug build; panics when the two
/// disagree, which a missed mark causes.
fn walked(
    refuses: bool,
    walk: impl FnOnce() -> Vec<ElementError>,
    checks: &str,
) -> Vec<ElementError> {
    if !refuses && !cfg!(debug_assertions) {
        return Vec::new();
    }
    let errors = walk();
    assert_eq!(
        refuses,
        !errors.is_empty(),
        "the kept {checks} checks say {refuses}, a walk over the tree finds {errors:?}: a change was not marked (docs/plans/iu3.md, PR 3a); reproduce with the failing test"
    );
    errors
}
