//! Taffy's work kept between updates (F2c S7). The box tree is rebuilt
//! each update; a box whose style, content and children are those of a
//! box from the last update, at the same display scale, with every
//! child unchanged too, takes that box's cache and layout, so Taffy
//! lays out only the changed boxes and their ancestors. Kept by diffing,
//! not by marks: a change a mark missed cannot leave a stale layout.

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

    /// Gives each unchanged box in `boxes` its kept cache and layout;
    /// returns how many.
    pub(crate) fn lend(&self, boxes: &mut [LayoutBox], scale: f32) -> usize {
        if self.boxes.is_empty() || self.scale.to_bits() != scale.to_bits() {
            return 0;
        }
        let old = keys(&self.boxes);
        let index: HashMap<Key, usize> = old.iter().enumerate().map(|(i, k)| (*k, i)).collect();
        let new = keys(boxes);
        let mut lent = 0;
        lend_from(boxes, 0, &new, &self.boxes, &old, &index, &mut lent);
        lent
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

/// Lends to `at` and below; whether `at`'s subtree was unchanged.
fn lend_from(
    boxes: &mut [LayoutBox],
    at: usize,
    new: &[Key],
    old_boxes: &[LayoutBox],
    old: &[Key],
    index: &HashMap<Key, usize>,
    lent: &mut usize,
) -> bool {
    let children = boxes[at].children.clone();
    let mut clean = true;
    for &child in &children {
        clean &= lend_from(boxes, child, new, old_boxes, old, index, lent);
    }
    let Some(&was) = index.get(&new[at]) else {
        return false;
    };
    let before = &old_boxes[was];
    let same_children = before.children.len() == children.len()
        && before
            .children
            .iter()
            .zip(&children)
            .all(|(o, n)| old[*o] == new[*n]);
    if !(clean
        && same_children
        && before.style == boxes[at].style
        && before.content == boxes[at].content)
    {
        return false;
    }
    boxes[at].cache = before.cache.clone();
    boxes[at].layout = before.layout;
    *lent += 1;
    true
}
