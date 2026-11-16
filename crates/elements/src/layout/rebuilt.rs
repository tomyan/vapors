//! What an update's box build changed, by slot (IU3 3c-3b,
//! `docs/plans/iu3.md` "Changed slots"): the slots a built box took,
//! the slots whose box is gone, and whether the tree's shape changed.
//! Paint patches what it keeps from these (`changed_slots.rs`).

use super::boxes::{Content, LayoutBox};
use super::isolated::Changed;
use super::owners::Build;
use super::reshaped::{Reshaped, reshaped};
use super::slots::{Key, Taken};
use crate::edit_decline::EditDecline;

/// The slots `KeptBoxes::rebuild` placed and freed.
#[derive(Debug)]
pub(crate) struct Rebuilt {
    /// Every box was built: what each slot held before is not known.
    pub(crate) every: bool,
    /// The slots a built box took, in no particular order.
    pub(crate) slots: Vec<usize>,
    /// The slots whose box is gone, with the key each held.
    pub(crate) freed: Vec<(usize, Key)>,
    /// A slot was freed or newly taken, or a rebuilt box's children
    /// differ from the box it replaced.
    pub(crate) shape_changed: bool,
    /// The slots whose box has other children than the box taken from
    /// it, each with the children it had: what the order splice reads
    /// (`order_splice.rs`).
    pub(crate) reshaped: Vec<Reshaped>,
    /// The owners whose rebuilt subtree changed, for layout to settle
    /// (`isolated.rs`); none when every box was built.
    pub(crate) changed: Vec<Changed>,
    /// Owners whose box children were edited in place
    /// (`child_edits.rs`), and those whose edits were declined.
    pub(crate) edits: EditCounts,
    /// The boxes lending the taken caches visited (`Lender::lend_from`).
    pub(crate) lend_visits: usize,
    /// Each owner edited in place, by slot, with the slots placement must
    /// chain to it (`Placement::chain_children`): the children it
    /// inserted, and each other owner rebuilt directly under it (a row
    /// restyled in the same update is an owner of its own, and may have
    /// come into flow while the edited owner kept it). Its kept children
    /// keep their slots and their parent, so these alone.
    pub(crate) to_chain: rustc_hash::FxHashMap<usize, Vec<usize>>,
    /// The child entries the rebuild searched or compared one by one:
    /// those `Placed::note_scanned` counted, and each old children list a
    /// reshaped slot compared (`reshaped`); 0 when the lanes answered.
    pub(crate) children_scanned: usize,
}

/// How many elements with recorded edits were edited, how many
/// declined, and the first decline: the owners' by slot, then the
/// elements that are no owner.
#[derive(Debug, Default)]
pub(crate) struct EditCounts {
    pub(crate) edited: usize,
    pub(crate) declined: usize,
    pub(crate) decline: Option<EditDecline>,
}

impl EditCounts {
    /// The counts of `builds`, in slot order.
    pub(crate) fn of<'a>(builds: impl Iterator<Item = &'a Build>) -> Self {
        let mut counts = Self::default();
        for build in builds {
            match build {
                Build::Edited(_) => counts.edited += 1,
                Build::Shallow(_, Some(decline)) => counts.count_decline(*decline),
                Build::Shallow(_, None) | Build::Full => {}
            }
        }
        counts
    }

    /// Counts the declines of the elements that are no owner, after
    /// the owners'.
    pub(crate) fn count_unowned(&mut self, declines: &[EditDecline]) {
        for &decline in declines {
            self.count_decline(decline);
        }
    }

    fn count_decline(&mut self, decline: EditDecline) {
        self.declined += 1;
        self.decline = self.decline.or(Some(decline));
    }
}

impl Rebuilt {
    /// Every box of `boxes` built.
    pub(crate) fn every(boxes: &[LayoutBox]) -> Self {
        let held = |(slot, layout_box): (usize, &LayoutBox)| {
            (!matches!(layout_box.content, Content::Vacant)).then_some(slot)
        };
        Self {
            every: true,
            slots: boxes.iter().enumerate().filter_map(held).collect(),
            freed: Vec::new(),
            shape_changed: true,
            reshaped: Vec::new(),
            changed: Vec::new(),
            edits: EditCounts::default(),
            lend_visits: 0,
            to_chain: rustc_hash::FxHashMap::default(),
            children_scanned: 0,
        }
    }

    /// The boxes put at `slots` in `boxes` in place of those `taken`
    /// out, once every subtree is put; an owner edited in place with the
    /// run its edits changed, `derived` from their places, not compared.
    pub(crate) fn subtrees(
        boxes: &[LayoutBox],
        slots: Vec<usize>,
        (taken, mut derived): (&Taken, rustc_hash::FxHashMap<usize, Reshaped>),
    ) -> Self {
        let freed = taken.unclaimed();
        let newly_taken = slots.iter().any(|slot| !taken.boxes.contains_key(slot));
        let mut compared = 0;
        let reshaped: Vec<Reshaped> = slots
            .iter()
            .filter_map(|&slot| run_of(boxes, slot, (taken, &mut derived), &mut compared))
            .collect();
        Self {
            every: false,
            shape_changed: !freed.is_empty() || newly_taken || !reshaped.is_empty(),
            slots,
            freed,
            reshaped,
            changed: Vec::new(),
            edits: EditCounts::default(),
            lend_visits: 0,
            to_chain: rustc_hash::FxHashMap::default(),
            children_scanned: compared,
        }
    }

    /// How many boxes were built.
    pub(crate) fn count(&self) -> usize {
        self.slots.len()
    }
}

/// The run of `slot`'s children that changed: `derived` from an edited
/// owner's edits (checked against a compare in debug builds), else
/// compared, its old children counted in `compared`.
fn run_of(
    boxes: &[LayoutBox],
    slot: usize,
    (taken, derived): (&Taken, &mut rustc_hash::FxHashMap<usize, Reshaped>),
    compared: &mut usize,
) -> Option<Reshaped> {
    if let Some(run) = derived.remove(&slot) {
        #[cfg(debug_assertions)]
        super::reshaped::assert_derived(boxes, &run, taken);
        return Some(run);
    }
    *compared += taken
        .boxes
        .get(&slot)
        .map_or(0, |(_, old)| old.children.len());
    reshaped(boxes, slot, taken)
}
