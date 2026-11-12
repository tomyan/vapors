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
        let mut nodes = self.marked();
        nodes.extend(self.added_subtrees());
        nodes
    }

    /// The box-marked and listener-marked elements still in the tree.
    fn marked(&self) -> Vec<NodeId> {
        let marked = self.boxes.marks.nodes();
        marked
            .chain(self.checks.marked.iter().copied())
            .filter(|&node| self.attached(node))
            .collect()
    }

    /// The elements of each subtree added since the last update and
    /// still in the tree.
    fn added_subtrees(&self) -> Vec<NodeId> {
        let added = self.checks.added.iter().copied();
        added
            .filter(|&node| self.attached(node))
            .flat_map(|node| elements(&self.arena, node))
            .collect()
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
        walked(&self.checks.tree, walk, "tree", |node| self.described(node))
    }

    /// The `style` attributes that do not parse, in document order.
    pub(crate) fn style_attribute_refusals(&self) -> Vec<ElementError> {
        let walk = || crate::check::style_attribute_refusals(&self.arena, self.root());
        let kept = &self.checks.style_attribute;
        walked(kept, walk, "style attribute", |node| self.described(node))
    }

    /// The refusals after styling, in document order.
    pub(crate) fn all_styled_refusals(&self, scale: f32) -> Vec<ElementError> {
        let walk = || self.styled_refusals(&elements(&self.arena, self.root()), scale);
        walked(&self.checks.styled, walk, "styled", |node| {
            self.described(node)
        })
    }

    fn described(&self, node: NodeId) -> String {
        describe(&self.arena, node)
    }

    /// Drops what is kept for `node` and its subtree, which leave the
    /// tree.
    pub(crate) fn forget_subtree(&mut self, node: NodeId) {
        for node in elements(&self.arena, node) {
            self.checks.forget(node);
            if let Some(kept) = self.kept.as_mut() {
                kept.styles.remove(node);
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
/// some element refuses, or always in a debug build, which panics when
/// the two disagree. A missed mark causes that; a release build, which
/// walks only when the kept checks say some element refuses, refuses
/// the update naming the element when the walk finds nothing.
fn walked(
    kept: &HashSet<NodeId>,
    walk: impl FnOnce() -> Vec<ElementError>,
    checks: &str,
    describe: impl Fn(NodeId) -> String,
) -> Vec<ElementError> {
    if kept.is_empty() && !cfg!(debug_assertions) {
        return Vec::new();
    }
    let errors = walk();
    debug_assert_eq!(
        !kept.is_empty(),
        !errors.is_empty(),
        "the kept {checks} checks say {kept:?} refuse, a walk over the tree finds {errors:?}: a change was not marked (docs/plans/iu3.md, PR 3a); reproduce with the failing test"
    );
    match kept.iter().min_by_key(|node| node.0) {
        Some(&node) if errors.is_empty() => vec![unmarked(&describe(node), checks)],
        _ => errors,
    }
}

/// The refusal when the kept checks say `element` refuses and a walk
/// over the tree finds no refusal.
fn unmarked(element: &str, checks: &str) -> ElementError {
    ElementError::Unmarked(format!(
        "{element}: the kept {checks} checks say it refuses and a check of the whole tree finds nothing: a change was not marked (docs/plans/iu3.md, PR 3a); reproduce in a debug build, which checks every update and panics at the change"
    ))
}

#[cfg(test)]
#[path = "style_checks_tests.rs"]
mod tests;
