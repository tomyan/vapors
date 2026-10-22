//! The thread-local runtime: both arenas plus the tracking, batching
//! and flushing state. Every public operation borrows it briefly and
//! releases it before running user code.

use std::cell::RefCell;
use std::panic::Location;

use crate::error::{Kind, NodeInfo};
use crate::node::{Arena, Node, NodeId, OwnerId, OwnerNode, State};

#[derive(Default)]
pub(crate) struct Runtime {
    pub nodes: Arena<Node>,
    pub owners: Arena<OwnerNode>,
    pub current_owner: Option<OwnerId>,
    /// Tracking stack: `Some(observer)` subscribes reads; `None` is an
    /// `untrack` frame.
    pub observers: Vec<Option<NodeId>>,
    /// Memos and effects currently computing, outermost first.
    pub computing: Vec<NodeId>,
    pub batch_depth: u32,
    pub flushing: bool,
    pub pending: Vec<NodeId>,
    next_seq: u64,
}

thread_local! {
    static RUNTIME: RefCell<Runtime> = RefCell::new(Runtime::default());
}

/// Borrows the runtime for the duration of `f`. `f` must not run user
/// code, which would re-borrow it.
pub(crate) fn with_runtime<R>(f: impl FnOnce(&mut Runtime) -> R) -> R {
    RUNTIME.with(|runtime| f(&mut runtime.borrow_mut()))
}

impl Runtime {
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.index, id.generation)
    }

    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(id.index, id.generation)
    }

    pub fn owner_mut(&mut self, id: OwnerId) -> Option<&mut OwnerNode> {
        self.owners.get_mut(id.index, id.generation)
    }

    /// Creates a node owned by the current owner.
    pub fn create_node(
        &mut self,
        kind: Kind,
        name: Option<&'static str>,
        created_at: &'static Location<'static>,
        state: State,
    ) -> NodeId {
        self.next_seq += 1;
        let owner = self.current_owner;
        let node = Node {
            seq: self.next_seq,
            info: NodeInfo {
                kind,
                name,
                created_at,
            },
            state,
            scheduled: false,
            value: None,
            compute: None,
            sources: Vec::new(),
            observers: Vec::new(),
            scope: None,
            owner,
        };
        let (index, generation) = self.nodes.insert(node);
        let id = NodeId { index, generation };
        if let Some(owner) = owner.and_then(|owner| self.owner_mut(owner)) {
            owner.nodes.push(id);
        }
        id
    }

    /// Creates an owner as a child of the current one.
    pub fn create_owner(&mut self) -> OwnerId {
        let parent = self.current_owner;
        let (index, generation) = self.owners.insert(OwnerNode {
            parent,
            children: Vec::new(),
            nodes: Vec::new(),
            cleanups: Vec::new(),
        });
        let id = OwnerId { index, generation };
        if let Some(parent) = parent.and_then(|parent| self.owner_mut(parent)) {
            parent.children.push(id);
        }
        id
    }
}
