//! The scene store: persistent nodes under one root, named by
//! generational ids.

use crate::handle::next_scene_id;
use crate::node::Node;
use crate::{CameraHandle, NodeId, NodeKind, StoreError, Transform};
use glam::Mat4;

#[derive(Debug)]
struct Slot {
    generation: u32,
    node: Option<Node>,
}

/// A tree of nodes. The root is a group made with the scene; every
/// other node has a parent.
#[derive(Debug)]
pub struct Scene {
    id: u64,
    slots: Vec<Slot>,
    free: Vec<u32>,
    root: NodeId,
    live: usize,
    pub(crate) dirty: Vec<NodeId>,
    pub(crate) camera: Option<CameraHandle>,
}

impl Scene {
    /// A scene holding only its root group, at the identity.
    #[must_use]
    pub fn new() -> Self {
        let id = next_scene_id();
        let mut scene = Self {
            id,
            slots: Vec::new(),
            free: Vec::new(),
            root: NodeId {
                scene: id,
                index: 0,
                generation: 0,
            },
            live: 0,
            dirty: Vec::new(),
            camera: None,
        };
        scene.root = scene.insert(NodeKind::Group, Transform::IDENTITY, None);
        scene
    }

    /// The root group.
    #[must_use]
    pub fn root(&self) -> NodeId {
        self.root
    }

    /// The number of live nodes, the root included.
    #[must_use]
    pub fn len(&self) -> usize {
        self.live
    }

    /// Always false: a scene holds at least its root.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Adds a node under `parent`; its world matrix is computed by the
    /// next [`Scene::update`].
    ///
    /// # Errors
    ///
    /// [`StoreError::Stale`] for a removed `parent`.
    pub fn add(
        &mut self,
        parent: NodeId,
        kind: NodeKind,
        local: Transform,
    ) -> Result<NodeId, StoreError> {
        self.node(parent)?;
        let id = self.insert(kind, local, Some(parent));
        self.node_mut(parent)?.children.push(id);
        Ok(id)
    }

    /// Removes `id` and its subtree, returning every id released; each
    /// is refused as stale from then on.
    ///
    /// # Errors
    ///
    /// [`StoreError::Stale`], or [`StoreError::RootFixed`] for the root.
    pub fn remove(&mut self, id: NodeId) -> Result<Vec<NodeId>, StoreError> {
        let parent = self.node(id)?.parent.ok_or(StoreError::RootFixed)?;
        self.node_mut(parent)?.children.retain(|child| *child != id);
        let released = self.subtree(id);
        for node in &released {
            self.release(*node);
        }
        Ok(released)
    }

    pub(crate) fn node(&self, id: NodeId) -> Result<&Node, StoreError> {
        self.check_scene(id)?;
        let slot = self
            .slots
            .get(id.index as usize)
            .filter(|slot| slot.generation == id.generation);
        slot.and_then(|slot| slot.node.as_ref())
            .ok_or(StoreError::Stale(id))
    }

    pub(crate) fn node_mut(&mut self, id: NodeId) -> Result<&mut Node, StoreError> {
        self.check_scene(id)?;
        let slot = self
            .slots
            .get_mut(id.index as usize)
            .filter(|slot| slot.generation == id.generation);
        slot.and_then(|slot| slot.node.as_mut())
            .ok_or(StoreError::Stale(id))
    }

    fn check_scene(&self, id: NodeId) -> Result<(), StoreError> {
        if id.scene == self.id {
            Ok(())
        } else {
            Err(StoreError::ForeignId(id))
        }
    }

    /// `id` and every node below it, parents before children.
    pub(crate) fn subtree(&self, id: NodeId) -> Vec<NodeId> {
        let mut nodes = vec![id];
        let mut next = 0;
        while let Some(node) = nodes.get(next).copied() {
            let node = self.node(node).expect("a live node's children are live");
            nodes.extend(node.children.iter().copied());
            next += 1;
        }
        nodes
    }

    /// Every live node with its id, in slot order.
    pub(crate) fn nodes(&self) -> impl Iterator<Item = (NodeId, &Node)> {
        self.slots.iter().zip(0u32..).filter_map(|(slot, index)| {
            let id = NodeId {
                scene: self.id,
                index,
                generation: slot.generation,
            };
            slot.node.as_ref().map(|node| (id, node))
        })
    }

    fn insert(&mut self, kind: NodeKind, local: Transform, parent: Option<NodeId>) -> NodeId {
        let node = Node {
            kind,
            local,
            parent,
            children: Vec::new(),
            world: Mat4::IDENTITY,
            dirty: false,
        };
        let index = self.free.pop().unwrap_or_else(|| self.push_slot());
        let slot = &mut self.slots[index as usize];
        slot.node = Some(node);
        self.live += 1;
        let id = NodeId {
            scene: self.id,
            index,
            generation: slot.generation,
        };
        self.mark(id);
        id
    }

    fn push_slot(&mut self) -> u32 {
        self.slots.push(Slot {
            generation: 0,
            node: None,
        });
        u32::try_from(self.slots.len() - 1).expect("a scene holds under 2^32 nodes")
    }

    fn release(&mut self, id: NodeId) {
        let slot = &mut self.slots[id.index as usize];
        slot.node = None;
        self.live -= 1;
        // A slot whose generation would wrap is retired, not reused: a
        // wrapped generation would hand out an id equal to a stale one.
        if let Some(generation) = slot.generation.checked_add(1) {
            slot.generation = generation;
            self.free.push(id.index);
        }
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
