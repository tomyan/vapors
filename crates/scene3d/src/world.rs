//! Local transforms, reparenting and world matrices composed parent to
//! child, recomputing only what changed.

use crate::{NodeId, Scene, StoreError, Transform};
use glam::Mat4;

impl Scene {
    /// Sets `id`'s local transform. The next [`Scene::update`]
    /// recomputes `id` and its descendants, and nothing else.
    ///
    /// # Errors
    ///
    /// [`StoreError::Stale`].
    pub fn set_transform(&mut self, id: NodeId, local: Transform) -> Result<(), StoreError> {
        self.node_mut(id)?.local = local;
        self.mark(id);
        Ok(())
    }

    /// `id`'s local transform.
    ///
    /// # Errors
    ///
    /// [`StoreError::Stale`].
    pub fn transform(&self, id: NodeId) -> Result<Transform, StoreError> {
        Ok(self.node(id)?.local)
    }

    /// Moves `id`, with its subtree, to the end of `parent`'s children.
    ///
    /// # Errors
    ///
    /// [`StoreError::Stale`] for either id, [`StoreError::RootFixed`]
    /// for the root, or [`StoreError::Cycle`] for a `parent` inside
    /// `id`'s subtree.
    pub fn reparent(&mut self, id: NodeId, parent: NodeId) -> Result<(), StoreError> {
        let old = self.node(id)?.parent.ok_or(StoreError::RootFixed)?;
        self.node(parent)?;
        if self.is_within(parent, id)? {
            return Err(StoreError::Cycle { node: id, parent });
        }
        self.node_mut(old)?.children.retain(|child| *child != id);
        self.node_mut(parent)?.children.push(id);
        self.node_mut(id)?.parent = Some(parent);
        self.mark(id);
        Ok(())
    }

    /// Recomputes the world matrix (`parent_world * local`) of every node
    /// whose transform or parent changed since the last update, and of
    /// their descendants. Returns those nodes, parents before children;
    /// its length is the number of matrices recomputed.
    pub fn update(&mut self) -> Vec<NodeId> {
        let marked = std::mem::take(&mut self.dirty);
        let tops: Vec<NodeId> = marked
            .into_iter()
            .filter(|id| self.is_top_mark(*id))
            .collect();
        let changed: Vec<NodeId> = tops.into_iter().flat_map(|id| self.subtree(id)).collect();
        for id in &changed {
            self.recompute(*id);
        }
        changed
    }

    /// `id`'s world matrix as of the last [`Scene::update`].
    ///
    /// # Errors
    ///
    /// [`StoreError::Stale`], or [`StoreError::WorldPending`] when `id`
    /// or an ancestor changed since that update.
    pub fn world(&self, id: NodeId) -> Result<Mat4, StoreError> {
        let mut at = Some(id);
        while let Some(node) = at.map(|id| self.node(id)).transpose()? {
            if node.dirty {
                return Err(StoreError::WorldPending(id));
            }
            at = node.parent;
        }
        Ok(self.node(id)?.world)
    }

    pub(crate) fn mark(&mut self, id: NodeId) {
        if let Ok(node) = self.node_mut(id)
            && !node.dirty
        {
            node.dirty = true;
            self.dirty.push(id);
        }
    }

    /// A marked node still live with no marked ancestor: its subtree
    /// covers every marked node below it.
    fn is_top_mark(&self, id: NodeId) -> bool {
        let Ok(node) = self.node(id) else {
            return false;
        };
        let mut at = node.parent;
        while let Some(ancestor) = at.and_then(|id| self.node(id).ok()) {
            if ancestor.dirty {
                return false;
            }
            at = ancestor.parent;
        }
        true
    }

    fn recompute(&mut self, id: NodeId) {
        let node = self.node(id).expect("update recomputes live nodes");
        let parent_world = node.parent.map_or(Mat4::IDENTITY, |parent| {
            self.node(parent)
                .expect("a live node's parent is live")
                .world
        });
        let world = parent_world * node.local.matrix();
        let node = self.node_mut(id).expect("update recomputes live nodes");
        node.world = world;
        node.dirty = false;
    }

    /// Whether `id` is `ancestor` or below it.
    fn is_within(&self, id: NodeId, ancestor: NodeId) -> Result<bool, StoreError> {
        let mut at = Some(id);
        while let Some(current) = at {
            if current == ancestor {
                return Ok(true);
            }
            at = self.node(current)?.parent;
        }
        Ok(false)
    }
}

#[cfg(test)]
#[path = "world_tests.rs"]
mod tests;
