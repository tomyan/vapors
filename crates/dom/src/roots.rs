//! What a component, branch or row owns at its top level: nodes placed
//! once, and blocks whose nodes change while they run. Removing or
//! moving the owner reads the blocks' current nodes at that moment, so
//! an `{if}` or `{for}` at the top of a branch, row or component goes
//! with it.

use std::fmt;
use std::rc::Rc;

use crate::NodeId;

/// One top-level entry.
#[derive(Debug, Clone)]
pub enum Root {
    /// A node placed once.
    Node(NodeId),
    /// An `{if}`, `{for}` or child component: its nodes as of now.
    Block(BlockNodes),
}

/// Reads a block's current top-level nodes, in document order, its
/// anchor last.
#[derive(Clone)]
pub struct BlockNodes(Rc<dyn Fn() -> Vec<NodeId>>);

impl BlockNodes {
    /// A handle over `nodes`, called each time the nodes are needed.
    pub fn new(nodes: impl Fn() -> Vec<NodeId> + 'static) -> BlockNodes {
        BlockNodes(Rc::new(nodes))
    }

    /// The block's top-level nodes now.
    #[must_use]
    pub fn nodes(&self) -> Vec<NodeId> {
        (self.0)()
    }
}

impl fmt::Debug for BlockNodes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("BlockNodes").field(&self.nodes()).finish()
    }
}

/// The current nodes of every root, in order.
#[must_use]
pub fn current_nodes(roots: &[Root]) -> Vec<NodeId> {
    let mut nodes = Vec::new();
    for root in roots {
        match root {
            Root::Node(node) => nodes.push(*node),
            Root::Block(block) => nodes.extend(block.nodes()),
        }
    }
    nodes
}
