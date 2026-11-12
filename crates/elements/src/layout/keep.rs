//! Taffy's work kept between updates (F2c S7, IU3). The boxes under
//! marked elements are built again (`rebuild`); a rebuilt box whose
//! style, content and children are those of the last update's box in
//! its slot, at the same display scale, with every child unchanged too,
//! takes that box's cache, layout and asked inputs, so Taffy lays out
//! only the changed boxes and their ancestors.
//!
//! One part of a style is not compared by value: a `calc()` with a %
//! term is a handle, the address of a computed `Calc`. An element
//! restyled this update has new computed styles, and a freed address
//! can be reused, so a changed calc could compare equal to the old
//! one. Such an element's box is lent nothing when its style holds a
//! calc (`may_keep`); an element not restyled keeps the very styles the
//! handles point into. An absolute or fixed box is never lent: its
//! containing block lies outside its subtree.

use taffy::Position;
use vapors_dom::NodeId;

use super::boxes::{Content, LayoutBox};
use super::marks::Marks;
use super::slots::{Key, Placed, Slots};
use crate::text_marks::TextMarks;

/// The boxes laid out by the last update, in their slots, at its scale.
#[derive(Default)]
pub(crate) struct KeptBoxes {
    pub(super) scale: f32,
    pub(super) placed: Placed,
    /// What changed since: the boxes the next update builds again.
    pub(crate) marks: Marks,
    /// The focused field's caret and selection the kept boxes show.
    pub(super) text_marks: Option<(NodeId, TextMarks)>,
    /// Debug builds skip `layout_check.rs` (`Document::skip_layout_check`).
    #[cfg(debug_assertions)]
    pub(crate) skip_layout_check: bool,
}

impl KeptBoxes {
    /// Keeps `placed`, laid out at `scale`, for the next update: the
    /// boxes `built` this update are kept from now, and every other box
    /// was kept already. Returns how many boxes it marked.
    ///
    /// # Panics
    /// A debug build, when a held box is not kept after: `built` is not
    /// every slot `rebuild` built.
    pub(crate) fn keep(&mut self, mut placed: Placed, scale: f32, built: &[usize]) -> usize {
        let mut marked = 0;
        for &slot in built {
            placed.boxes[slot].kept = true;
            marked += 1;
        }
        debug_assert!(
            placed
                .boxes
                .iter()
                .all(|b| b.kept || matches!(b.content, Content::Vacant)),
            "a box built this update is not among the built slots, so it is not kept: \
             `KeptBoxes::keep` is given a list other than `ChangedSlots::rebuilt`, or \
             `rebuild` reports a slot short; `cargo test -p vapors-elements` reproduces it"
        );
        (self.placed, self.scale) = (placed, scale);
        marked
    }

    /// `built`, boxes in pre-order, placed into the kept boxes' slots
    /// (`Slots::place`).
    pub(crate) fn place(&self, built: Vec<LayoutBox>) -> Placed {
        self.placed.slots.place(built)
    }

    /// The kept box of `node`, an element.
    pub(crate) fn element(&self, node: NodeId) -> Option<&LayoutBox> {
        let slot = self.placed.slots.of(Key::Element(node))?;
        Some(&self.placed.boxes[slot])
    }

    /// Gives each unchanged box in `placed` its kept cache and layout,
    /// except an element's for which `may_keep` says no; returns how
    /// many.
    pub(crate) fn lend(
        &self,
        placed: &mut Placed,
        scale: f32,
        may_keep: &dyn Fn(NodeId) -> bool,
    ) -> usize {
        let nothing = self.placed.boxes.is_empty() || placed.boxes.is_empty();
        if nothing || self.scale.to_bits() != scale.to_bits() {
            return 0;
        }
        let (old, old_boxes) = (&self.placed.slots, &self.placed.boxes);
        let before = |at: usize| Some((old.key(at)?, old_boxes.get(at)?));
        let mut lender = Lender {
            before: &before,
            new: &placed.slots,
            may_keep,
            lent: 0,
        };
        lender.lend_from(&mut placed.boxes, 0);
        lender.lent
    }
}

/// What lending compares against.
pub(super) struct Lender<'a> {
    /// The last update's key and box in a slot.
    pub(super) before: &'a dyn Fn(usize) -> Option<(Key, &'a LayoutBox)>,
    pub(super) new: &'a Slots,
    pub(super) may_keep: &'a dyn Fn(NodeId) -> bool,
    pub(super) lent: usize,
}

impl Lender<'_> {
    /// Lends to `at` and below; whether `at`'s subtree was unchanged.
    /// A key keeps its slot, so a box is unchanged only where the last
    /// update's box in its slot had its key, and its children are
    /// unchanged when they hold the same slots and are unchanged too.
    /// A kept box, which a shallow owner kept in its slot, keeps its
    /// cache and layout.
    pub(super) fn lend_from(&mut self, boxes: &mut [LayoutBox], at: usize) -> bool {
        if boxes[at].kept {
            return true;
        }
        let children = boxes[at].children.clone();
        let mut clean = true;
        for &child in &children {
            clean &= self.lend_from(boxes, child);
        }
        let Some((key, before)) = (self.before)(at) else {
            return false;
        };
        if Some(key) != self.new.key(at) {
            return false;
        }
        // An absolute or fixed box is placed against a containing block
        // outside its subtree, which may have changed, so it is laid
        // out again, and with it every ancestor up to that block, which
        // then hoists it afresh (Taffy calls add_hoisted_children only
        // for a block it lays out).
        let in_flow = !matches!(
            boxes[at].style.position,
            Position::Absolute | Position::Fixed
        );
        let keepable = in_flow
            && match boxes[at].content {
                Content::Element(node) => (self.may_keep)(node),
                Content::Text(_) => true,
                Content::Vacant => unreachable!("slot {at} is vacant, and no box's child"),
            };
        if !(clean
            && before.children == children
            && keepable
            && before.style == boxes[at].style
            && before.content == boxes[at].content
            && before.natural == boxes[at].natural)
        {
            return false;
        }
        boxes[at].cache = before.cache.clone();
        boxes[at].layout = before.layout;
        boxes[at].asked.clone_from(&before.asked);
        self.lent += 1;
        true
    }
}

#[cfg(test)]
#[path = "keep_tests.rs"]
mod tests;
