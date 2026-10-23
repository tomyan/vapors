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
//! handles point into.

use std::collections::HashMap;

use vapors_dom::NodeId;

use super::boxes::{Content, LayoutBox};

/// Which box a box is across updates: an element's, or the text box
/// at a position among its parent's text boxes.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
enum Key {
    Element(NodeId),
    Text(NodeId, usize),
}

/// The boxes laid out by the last update, at its scale.
#[derive(Default)]
pub(crate) struct KeptBoxes {
    scale: f32,
    boxes: Vec<LayoutBox>,
}

impl KeptBoxes {
    /// Keeps `boxes`, laid out at `scale`, for the next update.
    pub(crate) fn keep(&mut self, boxes: Vec<LayoutBox>, scale: f32) {
        *self = Self { scale, boxes };
    }

    /// Gives each unchanged box in `boxes` its kept cache and layout,
    /// except an element's for which `may_keep` says no; returns how
    /// many.
    pub(crate) fn lend(
        &self,
        boxes: &mut [LayoutBox],
        scale: f32,
        may_keep: &dyn Fn(NodeId) -> bool,
    ) -> usize {
        if self.boxes.is_empty() || self.scale.to_bits() != scale.to_bits() {
            return 0;
        }
        let old = keys(&self.boxes);
        let mut lender = Lender {
            index: old.iter().enumerate().map(|(i, k)| (*k, i)).collect(),
            old,
            new: keys(boxes),
            old_boxes: &self.boxes,
            may_keep,
            lent: 0,
        };
        lender.lend_from(boxes, 0);
        lender.lent
    }
}

/// Every box's key, by index.
fn keys(boxes: &[LayoutBox]) -> Vec<Key> {
    let mut keys = vec![Key::Text(NodeId(0), 0); boxes.len()];
    for (index, layout_box) in boxes.iter().enumerate() {
        if let Content::Element(node) = layout_box.content {
            keys[index] = Key::Element(node);
            let mut texts = 0;
            for &child in &layout_box.children {
                if matches!(boxes[child].content, Content::Text(_)) {
                    keys[child] = Key::Text(node, texts);
                    texts += 1;
                }
            }
        }
    }
    keys
}

/// What lending compares against.
struct Lender<'a> {
    new: Vec<Key>,
    old: Vec<Key>,
    index: HashMap<Key, usize>,
    old_boxes: &'a [LayoutBox],
    may_keep: &'a dyn Fn(NodeId) -> bool,
    lent: usize,
}

impl Lender<'_> {
    /// Lends to `at` and below; whether `at`'s subtree was unchanged.
    fn lend_from(&mut self, boxes: &mut [LayoutBox], at: usize) -> bool {
        let children = boxes[at].children.clone();
        let mut clean = true;
        for &child in &children {
            clean &= self.lend_from(boxes, child);
        }
        let Some(&was) = self.index.get(&self.new[at]) else {
            return false;
        };
        let before = &self.old_boxes[was];
        let same_children = before.children.len() == children.len()
            && before
                .children
                .iter()
                .zip(&children)
                .all(|(o, n)| self.old[*o] == self.new[*n]);
        let keepable = match boxes[at].content {
            Content::Element(node) => (self.may_keep)(node),
            Content::Text(_) => true,
        };
        if !(clean
            && same_children
            && keepable
            && before.style == boxes[at].style
            && before.content == boxes[at].content)
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
        kept.keep(with_calc(0x1000), 1.0);
        let mut boxes = with_calc(0x1000);

        // When: the element was restyled, its calc may have changed
        let lent = kept.lend(&mut boxes, 1.0, &|_| false);

        // Then
        assert_eq!(lent, 0);
    }

    #[test]
    fn an_unrestyled_box_holding_a_calc_is_lent_its_cache() {
        // Given
        let mut kept = KeptBoxes::default();
        kept.keep(with_calc(0x1000), 1.0);
        let mut boxes = with_calc(0x1000);

        // When
        let lent = kept.lend(&mut boxes, 1.0, &|_| true);

        // Then
        assert_eq!(lent, 1);
    }
}
