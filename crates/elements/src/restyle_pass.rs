//! The restyle pass: the marked subtrees and elements restyled against
//! the styles kept for their parents (see `restyle`).

use std::collections::{HashMap, HashSet};

use vapors_dom::NodeId;

use crate::check::elements;
use crate::document::Document;
use crate::error::ElementError;
use crate::restyle::{Dirty, Kept};
use crate::style::{Context, ElementStyles, Styles, used};
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
        self.dirty = Dirty::default();
        self.kept
            .as_mut()
            .expect("set above or by an earlier update")
            .restyled_nodes
            .clear();
        match self.restyle_all(&roots, &singles, viewport) {
            Ok(count) => {
                self.kept.as_mut().expect("set above").restyled = count;
                Ok(())
            }
            Err(error) => {
                self.dirty.everything();
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
            used: HashMap::new(),
            refused: HashMap::new(),
            restyled: 0,
            restyled_nodes: HashSet::new(),
        }
    }

    /// The marked subtree roots still in the tree, none inside another,
    /// and the marked elements inside none of them, parents first.
    fn dirty_roots(&self) -> (Vec<NodeId>, Vec<NodeId>) {
        let live: HashSet<NodeId> = elements(&self.arena, self.root()).into_iter().collect();
        let marked: HashSet<NodeId> = self
            .dirty
            .roots
            .iter()
            .copied()
            .filter(|n| live.contains(n))
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
            .filter(|n| live.contains(n) && !covered(*n, 0))
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

    /// Computes and keeps `node`'s styles; whether its computed style
    /// changed.
    fn restyle_one(&mut self, node: NodeId, viewport: Viewport) -> Result<bool, ElementError> {
        let kept = self.kept.as_ref().expect("restyle sets it");
        let parent = self.parent(node).map(|p| kept.styles.of(p));
        let style =
            self.sheets
                .compute_element(&self.arena, node, parent, &environment(viewport))?;
        let changed = kept.styles.get(node) != Some(&style);
        let kept = self.kept.as_mut().expect("restyle sets it");
        kept.styles.set(node, style);
        kept.restyled_nodes.insert(node);
        self.keep_used(node, viewport);
        Ok(changed)
    }

    /// Keeps `node`'s used style, or its refusals, from its kept computed
    /// style and its parent's.
    fn keep_used(&mut self, node: NodeId, viewport: Viewport) {
        if !crate::check::implemented(&self.arena, node) {
            return;
        }
        let kept = self.kept.as_ref().expect("restyle sets it");
        let parent_node = self.parent(node);
        let parent_used = parent_node.and_then(|p| kept.used.get(&p));
        let context = Context {
            initial: &kept.initial,
            flex_item: parent_used.is_some_and(|p| {
                matches!(
                    p.layout.display,
                    taffy::Display::Flex | taffy::Display::Grid
                )
            }),
            in_inline: parent_used.is_some_and(|p| p.inline),
            to_viewport: crate::style::to_viewport(
                node == self.root(),
                node == self.body(),
                kept.used.get(&self.root()).is_none_or(|root| !root.clips()),
            ),
            scale: viewport.scale,
        };
        let styles = ElementStyles {
            own: kept.styles.of(node),
            parent: parent_node.map(|p| kept.styles.of(p)),
        };
        let result = used(styles, &context);
        let refused: Vec<ElementError> = match &result {
            Ok(_) => Vec::new(),
            Err(refusals) => self.unhonoured(node, refusals.clone()).collect(),
        };
        let kept = self.kept.as_mut().expect("restyle sets it");
        kept.refused.insert(node, refused);
        match result {
            Ok(style) => drop(kept.used.insert(node, style)),
            Err(_) => drop(kept.used.remove(&node)),
        }
    }
}
