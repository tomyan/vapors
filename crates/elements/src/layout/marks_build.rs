//! How each children-marked element is built: as an owner, edited in
//! place or shallow; or by an ancestor, when it makes no box of its
//! own, with its recorded edits declined and counted (review of #285,
//! M1); or not at all as its own thing, when it held no box last
//! update and so had nothing to edit.

use rustc_hash::FxHashMap;
use vapors_dom::NodeId;

use super::{Build, Marks, Resolver};
use crate::edit_decline::EditDecline;

/// What a children mark on an element comes to.
enum Marked {
    /// It held a slot and makes a box: an owner.
    Owner(Build),
    /// It held a slot but makes no box now (hidden in this update): an
    /// ancestor builds it, and its edits are declined.
    NoBox(EditDecline),
    /// It held no slot: new, hidden since before the last update, or
    /// removed from the document. Nothing to edit, nothing declined.
    NoSlot,
}

impl Marks {
    /// The children-marked elements that are owners, each edited in
    /// place where its recorded edits allow, else with its kept
    /// children; and the declines of those that are no owner.
    pub(super) fn shallow(
        &self,
        resolver: &Resolver<'_>,
    ) -> (FxHashMap<NodeId, Build>, Vec<EditDecline>) {
        let mut shallow = FxHashMap::default();
        let mut unowned = Vec::new();
        for &node in &self.children {
            match self.build_of(resolver, node) {
                Marked::Owner(build) => _ = shallow.insert(node, build),
                Marked::NoBox(decline) => unowned.push(decline),
                Marked::NoSlot => {}
            }
        }
        (shallow, unowned)
    }

    fn build_of(&self, resolver: &Resolver<'_>, node: NodeId) -> Marked {
        let edited = self
            .edits
            .get(&node)
            .map(|edits| resolver.edits(node, edits));
        let decline = match edited {
            Some(Ok(edits)) => return Marked::Owner(Build::Edited(edits)),
            Some(Err(decline)) => Some(decline),
            None => None,
        };
        match resolver.kept_children(node) {
            Some(kept) => Marked::Owner(Build::Shallow(kept, decline)),
            None => no_box(resolver, node, decline),
        }
    }
}

/// `node` makes no box of its own. `kept_children` is `None` on the
/// same condition `Resolver::edits` declines as `OwnerHasNoBox`
/// (`Resolver::slot`); the assertion holds the two together.
fn no_box(resolver: &Resolver<'_>, node: NodeId, decline: Option<EditDecline>) -> Marked {
    assert_eq!(
        decline,
        Some(EditDecline::OwnerHasNoBox),
        "{node:?} has no kept children but its edits were not declined for it: \
         `Resolver::kept_children` and `Resolver::edits` disagree on `slot`"
    );
    if resolver.held_slot(node) {
        Marked::NoBox(EditDecline::OwnerHasNoBox)
    } else {
        Marked::NoSlot
    }
}
