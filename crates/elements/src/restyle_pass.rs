//! The restyle pass: the marked subtrees and elements restyled against
//! the styles kept for their parents (see `restyle`).

use std::collections::HashSet;

use crate::node_map::{NodeMap, NodeSet};
use crate::style::UsedMap;

use vapors_dom::NodeId;

use crate::check::elements;
use crate::document::Document;
use crate::error::ElementError;
use crate::restyle::{Dirty, Kept};
use crate::style::Styles;
use crate::update::{Viewport, environment};

impl Document {
    /// Restyles what is marked, or everything when the viewport or the
    /// sheets changed; keeps the result and how many were restyled.
    ///
    /// # Errors
    /// A computed style that does not compute (an unresolved `var()`);
    /// the next update then restyles everything.
    pub(crate) fn restyle(&mut self, viewport: Viewport) -> Result<(), Vec<ElementError>> {
        let fresh = self.dirty.all || self.kept.as_ref().is_none_or(|k| k.viewport != viewport);
        let (roots, singles) = if fresh {
            self.kept = Some(self.fresh_kept(viewport));
            (vec![self.root()], Vec::new())
        } else {
            self.dirty_roots()
        };
        // Debug builds restyle the whole document again after an element
        // insert or remove and compare it with the narrowed result
        // (`restyle_check.rs`): O(elements) there, though
        // `elements_restyled` still counts the narrowed set. Release
        // builds skip it.
        let check_narrowed = cfg!(debug_assertions) && self.dirty.narrowed && !fresh;
        self.dirty = Dirty::default();
        self.kept
            .as_mut()
            .expect("set above or by an earlier update")
            .forget_restyled();
        match self.restyle_all(&roots, &singles, viewport) {
            Ok(count) => {
                self.counts.elements_restyled = count;
                if check_narrowed {
                    self.check_narrowed_restyle(viewport);
                }
                let kept = self.kept.as_mut().expect("set above");
                kept.shared.sweep();
                self.counts.styles_live = kept.styles.live();
                Ok(())
            }
            Err(error) => {
                self.dirty.everything();
                self.counts.forget_styles();
                Err(vec![error])
            }
        }
    }

    fn restyle_all(
        &mut self,
        roots: &[NodeId],
        singles: &[NodeId],
        viewport: Viewport,
    ) -> Result<usize, ElementError> {
        let mut count = 0;
        for &root in roots {
            count += self.restyle_subtree(root, viewport)?;
        }
        for &node in singles {
            count += self.restyle_element(node, viewport)?;
        }
        Ok(count)
    }

    fn fresh_kept(&self, viewport: Viewport) -> Kept {
        Kept {
            viewport,
            initial: crate::style::initial_style(&self.arena, self.root(), &environment(viewport)),
            styles: Styles::default(),
            used: UsedMap::default(),
            scene: NodeMap::default(),
            refused: NodeMap::default(),
            restyled_nodes: NodeSet::default(),
            calcs: crate::layout::Calcs::default(),
            shared: crate::style::Shared::default(),
        }
    }

    /// The marked subtree roots still in the tree, none inside another,
    /// and the marked elements inside none of them, parents first.
    fn dirty_roots(&self) -> (Vec<NodeId>, Vec<NodeId>) {
        let marked: HashSet<NodeId> = self
            .dirty
            .roots
            .iter()
            .copied()
            .filter(|&n| self.attached(n))
            .collect();
        let covered = |node: NodeId, skip: usize| {
            self.chain(node)
                .iter()
                .skip(skip)
                .any(|a| marked.contains(a))
        };
        let mut roots: Vec<NodeId> = marked.iter().copied().filter(|n| !covered(*n, 1)).collect();
        roots.sort_by_key(|node| node.0);
        let mut singles: Vec<NodeId> = self
            .dirty
            .elements
            .iter()
            .copied()
            .filter(|&n| self.attached(n) && !covered(n, 0))
            .collect();
        singles.sort_by_key(|node| (self.chain(*node).len(), node.0));
        singles.dedup();
        (roots, singles)
    }

    /// Restyles `root` and every element below it, parents first;
    /// returns how many.
    fn restyle_subtree(&mut self, root: NodeId, viewport: Viewport) -> Result<usize, ElementError> {
        let nodes = elements(&self.arena, root);
        for &node in &nodes {
            self.restyle_one(node, viewport)?;
        }
        Ok(nodes.len())
    }

    /// Restyles `node` alone; when its style changed, its children's
    /// subtrees too, since what they inherit may have. Returns how many.
    fn restyle_element(&mut self, node: NodeId, viewport: Viewport) -> Result<usize, ElementError> {
        if !self.restyle_one(node, viewport)? {
            return Ok(1);
        }
        let mut count = 1;
        for child in self.arena.children(node) {
            if self.arena.get(child, "restyle").element().is_some() {
                count += self.restyle_subtree(child, viewport)?;
            }
        }
        Ok(count)
    }

    /// Finds and keeps `node`'s styles (`restyle_share`); whether its
    /// computed style changed.
    fn restyle_one(&mut self, node: NodeId, viewport: Viewport) -> Result<bool, ElementError> {
        let (style, path) = self.style_for(node, viewport)?;
        self.counts.count_style(path);
        let kept = self.kept.as_mut().expect("restyle sets it");
        let changed = match kept.styles.rc(node) {
            Some(old) if std::rc::Rc::ptr_eq(old, &style) => false,
            old => old.map(|o| &***o) != Some(&**style),
        };
        kept.styles.set(node, style);
        kept.restyled_nodes.insert(node);
        // a box holding a calc handle points into the replaced style
        if changed || !kept.may_keep_layout(node) {
            self.boxes.marks.mark(node);
        }
        self.keep_used(node, viewport, path);
        Ok(changed)
    }
}
