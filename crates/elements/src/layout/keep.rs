//! Taffy's work kept between updates (F2c S7). The box tree is rebuilt
//! each update; a box whose style, content and children are those of a
//! box from the last update, at the same display scale, with every
//! child unchanged too, takes that box's cache and layout, so Taffy
//! lays out only the changed boxes and their ancestors. Kept by diffing,
//! not by marks: a change a mark missed cannot leave a stale layout.
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
use super::slots::{Key, Placed, Slots};

/// The boxes laid out by the last update, in their slots, at its scale.
#[derive(Default)]
pub(crate) struct KeptBoxes {
    scale: f32,
    placed: Placed,
}

impl KeptBoxes {
    /// Keeps `placed`, laid out at `scale`, for the next update.
    pub(crate) fn keep(&mut self, placed: Placed, scale: f32) {
        *self = Self { scale, placed };
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
        if self.placed.boxes.is_empty() || self.scale.to_bits() != scale.to_bits() {
            return 0;
        }
        let mut lender = Lender {
            old: &self.placed.slots,
            new: &placed.slots,
            old_boxes: &self.placed.boxes,
            may_keep,
            lent: 0,
        };
        lender.lend_from(&mut placed.boxes, 0);
        lender.lent
    }
}

/// What lending compares against.
struct Lender<'a> {
    old: &'a Slots,
    new: &'a Slots,
    old_boxes: &'a [LayoutBox],
    may_keep: &'a dyn Fn(NodeId) -> bool,
    lent: usize,
}

impl Lender<'_> {
    /// Lends to `at` and below; whether `at`'s subtree was unchanged.
    /// A key keeps its slot, so a box is unchanged only where the last
    /// update's box in its slot had its key, and its children are
    /// unchanged when they hold the same slots and are unchanged too.
    fn lend_from(&mut self, boxes: &mut [LayoutBox], at: usize) -> bool {
        let children = boxes[at].children.clone();
        let mut clean = true;
        for &child in &children {
            clean &= self.lend_from(boxes, child);
        }
        if self.old.key(at) != self.new.key(at) {
            return false;
        }
        let before = &self.old_boxes[at];
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
                Content::Text(_) | Content::Vacant => true,
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
        self.lent += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use taffy::{Dimension, Size, Style};

    use super::*;

    /// One element box whose width is a calc handle at `address`.
    fn with_calc(address: usize) -> Vec<LayoutBox> {
        let style = Style {
            size: Size {
                width: Dimension::calc(address as *const ()),
                height: Dimension::auto(),
            },
            ..Style::default()
        };
        vec![LayoutBox::new(Content::Element(NodeId(7)), style)]
    }

    #[test]
    fn a_restyled_box_holding_a_calc_is_lent_nothing_even_when_the_address_matches() {
        // Given: the last update's box and this one's share a handle
        // address, as a freed and reused allocation would make them
        let mut kept = KeptBoxes::default();
        kept.keep(Slots::default().place(with_calc(0x1000)), 1.0);
        let mut placed = kept.place(with_calc(0x1000));

        // When: the element was restyled, its calc may have changed
        let lent = kept.lend(&mut placed, 1.0, &|_| false);

        // Then
        assert_eq!(lent, 0);
    }

    #[test]
    fn an_unrestyled_box_holding_a_calc_is_lent_its_cache() {
        // Given
        let mut kept = KeptBoxes::default();
        kept.keep(Slots::default().place(with_calc(0x1000)), 1.0);
        let mut placed = kept.place(with_calc(0x1000));

        // When
        let lent = kept.lend(&mut placed, 1.0, &|_| true);

        // Then
        assert_eq!(lent, 1);
    }
}
