//! Where marks resolve (IU3, `docs/plans/splice.md` (b)): a box mark to
//! its owner, whose subtree is built again; a children mark on an
//! element with a slot and a box of its own to a shallow owner, which
//! builds its own box and runs and the children its kept box did not
//! have, and keeps the others in their slots.

use std::collections::HashSet;

use rustc_hash::FxHashMap;
use taffy::Display;
use vapors_dom::NodeId;

use super::child_edits::Edits;
use super::slots::{Key, Placed};
use crate::arena::Arena;
use crate::edit_decline::EditDecline;
use crate::style::UsedMap;

/// A shallow owner's kept children: each element child whose box was a
/// child of the owner's kept box and which still makes a box, with its
/// slot.
pub(crate) type KeptChildren = FxHashMap<NodeId, usize>;

/// What an update builds again.
#[derive(Debug, PartialEq)]
pub(crate) enum Rebuild {
    /// Every box.
    Every,
    /// These owners' boxes, in the slots they hold; and the declines
    /// of the children-marked elements that are no owner, having no
    /// box of their own (`EditDecline::OwnerHasNoBox`).
    Owners(Vec<Owner>, Vec<EditDecline>),
}

/// An element whose boxes an update builds again.
#[derive(Debug, PartialEq)]
pub(crate) struct Owner {
    pub(crate) node: NodeId,
    pub(crate) build: Build,
}

/// How an owner's boxes are built.
#[derive(Debug, PartialEq)]
pub(crate) enum Build {
    /// The whole subtree.
    Full,
    /// Its own box and runs, and the children not kept in their slots;
    /// with why its recorded edits were not made as an edit, if any.
    Shallow(KeptChildren, Option<EditDecline>),
    /// Its kept box's children edited in place (`child_edits.rs`): the
    /// removed taken out, the inserted built.
    Edited(Edits),
}

impl Build {
    /// Whether this owner builds its child `child`'s boxes.
    fn builds(&self, child: NodeId) -> bool {
        match self {
            Self::Full => true,
            Self::Shallow(kept, _) => !kept.contains_key(&child),
            Self::Edited(edits) => edits.inserted.contains(&child),
        }
    }
}

/// Where a box mark resolves.
pub(super) enum Resolved {
    /// To this element, which builds the marked node's boxes.
    Element(NodeId),
    /// To no element: the walk reached the root without one.
    None,
    /// Nowhere: the node is freed or not under the root.
    Detached,
}

/// What resolving reads: the tree, the used styles, the kept boxes.
pub(super) struct Resolver<'a> {
    pub(super) arena: &'a Arena,
    pub(super) used: &'a UsedMap,
    pub(super) placed: &'a Placed,
    pub(super) root: NodeId,
}

impl Resolver<'_> {
    /// The nearest element at or above `node` with a slot and a box of
    /// its own.
    pub(super) fn owner(&self, node: NodeId) -> Resolved {
        let mut at = Some(node).filter(|&node| self.arena.contains(node));
        while let Some(node) = at {
            if self.slot(node).is_some() {
                return Resolved::Element(node);
            }
            if node == self.root {
                return Resolved::None;
            }
            at = self.arena.get(node, "owner").parent;
        }
        Resolved::Detached
    }

    /// `node`'s kept children, when it is in the tree, makes a box of
    /// its own and holds a slot.
    pub(super) fn kept_children(&self, node: NodeId) -> Option<KeptChildren> {
        let slot = self.slot(Some(node).filter(|&n| self.arena.contains(n))?)?;
        let kept = |&child: &usize| match self.placed.slots.key(child) {
            Some(Key::Element(element)) if self.kept_child(element, node) => Some((element, child)),
            _ => None,
        };
        Some(
            self.placed.boxes[slot]
                .children
                .iter()
                .filter_map(kept)
                .collect(),
        )
    }

    /// Whether `element`, whose box was a child of `owner`'s, is still
    /// an element child of `owner` that makes a box of its own.
    fn kept_child(&self, element: NodeId, owner: NodeId) -> bool {
        self.arena.contains(element)
            && self.arena.get(element, "kept_child").parent == Some(owner)
            && boxed(self.used, element)
    }

    /// Whether `node` is in the tree and held a box slot last update,
    /// whether or not it makes a box now.
    pub(super) fn held_slot(&self, node: NodeId) -> bool {
        self.arena.contains(node) && self.placed.slots.of(Key::Element(node)).is_some()
    }

    /// `node`'s slot, when it makes a box of its own.
    pub(super) fn slot(&self, node: NodeId) -> Option<usize> {
        let slot = self.placed.slots.of(Key::Element(node));
        slot.filter(|_| boxed(self.used, node))
    }

    /// The owners under `root` that no other owner builds, by slot.
    pub(super) fn outermost(
        &self,
        full: &HashSet<NodeId>,
        mut shallow: FxHashMap<NodeId, Build>,
    ) -> Vec<Owner> {
        let outer = |&node: &NodeId| self.placement(node, full, &shallow) == Placement::Outermost;
        let full_outer: Vec<NodeId> = full.iter().copied().filter(outer).collect();
        let shallow_outer: Vec<NodeId> = shallow.keys().copied().filter(outer).collect();
        let fulls = full_outer.into_iter().map(|node| Owner {
            node,
            build: Build::Full,
        });
        let shallows = shallow_outer.into_iter().filter_map(|node| {
            let build = shallow.remove(&node)?;
            Some(Owner { node, build })
        });
        let mut owners: Vec<Owner> = fulls.chain(shallows).collect();
        owners.sort_by_key(|owner| self.placed.slots.of(Key::Element(owner.node)));
        owners
    }

    /// Where `node` sits among the owners: inside a full owner, or
    /// inside a shallow owner's child that it builds, is built there.
    fn placement(
        &self,
        node: NodeId,
        full: &HashSet<NodeId>,
        shallow: &FxHashMap<NodeId, Build>,
    ) -> Placement {
        let mut at = node;
        while let Some(ancestor) = self.arena.get(at, "owner").parent {
            let builds = |build: &Build| build.builds(at);
            if full.contains(&ancestor) || shallow.get(&ancestor).is_some_and(builds) {
                return Placement::Inside;
            }
            at = ancestor;
        }
        if at == self.root {
            Placement::Outermost
        } else {
            Placement::Detached
        }
    }
}

/// Where an owner sits among the others.
#[derive(PartialEq)]
enum Placement {
    /// Under `root`, with no owner above it that builds its boxes.
    Outermost,
    /// Under another owner, which builds its boxes.
    Inside,
    /// Not under `root`: nothing shows it.
    Detached,
}

/// Whether `node` makes a box of its own: not `display: none` and not
/// inline.
fn boxed(used: &UsedMap, node: NodeId) -> bool {
    used.get(&node)
        .is_some_and(|style| style.layout.display != Display::None && !style.inline)
}
