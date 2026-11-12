//! What the scene store refuses, each naming the fix.

use crate::{CameraHandle, NodeId};
use std::fmt;

/// An operation the scene store cannot carry out.
#[derive(Debug, Clone, PartialEq)]
pub enum StoreError {
    /// An id whose node was removed.
    Stale(NodeId),
    /// An id another scene returned.
    ForeignId(NodeId),
    /// A remove or reparent of the root, which has no parent.
    RootFixed,
    /// A node moved under itself or one of its descendants.
    Cycle {
        /// The node being moved.
        node: NodeId,
        /// The parent asked for, inside `node`'s subtree.
        parent: NodeId,
    },
    /// The scene root names no camera.
    NoCamera,
    /// No camera node holds the handle the root names.
    CameraUnresolved(CameraHandle),
    /// More than one camera node holds the handle the root names.
    CameraDuplicate {
        /// The handle.
        handle: CameraHandle,
        /// The camera nodes holding it.
        nodes: Vec<NodeId>,
    },
    /// A world matrix asked for before [`crate::Scene::update`] ran on
    /// the node's latest transform.
    WorldPending(NodeId),
    /// A mesh's setter called on a group or camera.
    NotMesh(NodeId),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stale(id) => write!(
                f,
                "{id} was removed: use an id this scene returned since its removal"
            ),
            Self::ForeignId(id) => write!(
                f,
                "{id} belongs to another scene: use ids returned by the scene you call"
            ),
            Self::RootFixed => write!(
                f,
                "the scene root cannot be removed or moved: remove or move its children instead"
            ),
            Self::Cycle { node, parent } => fmt_cycle(f, *node, *parent),
            Self::NoCamera => write!(
                f,
                "the scene names no active camera: call Scene::set_camera with a camera's handle"
            ),
            Self::CameraUnresolved(handle) => fmt_unresolved(f, *handle),
            Self::CameraDuplicate { handle, nodes } => fmt_duplicate(f, *handle, nodes),
            Self::WorldPending(id) => write!(
                f,
                "{id}'s world matrix is out of date: call Scene::update after changing transforms"
            ),
            Self::NotMesh(id) => write!(
                f,
                "{id} is not a mesh, so Scene::set_colour cannot colour it: pass a mesh node's id"
            ),
        }
    }
}

fn fmt_cycle(f: &mut fmt::Formatter<'_>, node: NodeId, parent: NodeId) -> fmt::Result {
    write!(
        f,
        "{node} cannot move under {parent}, which is inside its own subtree: pick a parent outside it"
    )
}

fn fmt_unresolved(f: &mut fmt::Formatter<'_>, handle: CameraHandle) -> fmt::Result {
    write!(
        f,
        "the scene names {handle}, which no camera node holds: add a camera with that handle or name one that exists"
    )
}

fn fmt_duplicate(
    f: &mut fmt::Formatter<'_>,
    handle: CameraHandle,
    nodes: &[NodeId],
) -> fmt::Result {
    let names: Vec<String> = nodes.iter().map(ToString::to_string).collect();
    write!(
        f,
        "the scene names {handle}, which cameras {} all hold: give each camera its own handle",
        names.join(", ")
    )
}

impl std::error::Error for StoreError {}
