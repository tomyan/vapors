//! What a box takes from the boxes around it by containing block (F2c
//! S8, S9): each box takes a value from its containing block's content
//! (its parent's for a box in flow, the box it is placed against for an
//! out-of-flow one, the root for a fixed one), and gives its own content
//! a value from that. Clips and scroll containers both pass this way.

use crate::layout::{LayoutBox, containing_blocks};

/// Each box's own value, box 0 taking `outside`; `content(index, own)`
/// is what box `index` gives its content. Ancestors, and so containing
/// blocks, are visited first.
pub(crate) fn by_containing_block<T: Copy>(
    boxes: &[LayoutBox],
    outside: T,
    content: &dyn Fn(usize, T) -> T,
) -> Vec<T> {
    let mut walk = Walk {
        boxes,
        content,
        containing: containing_blocks(boxes),
        own: vec![outside; boxes.len()],
        inner: vec![outside; boxes.len()],
    };
    walk.visit(0, outside);
    walk.own
}

struct Walk<'a, T> {
    boxes: &'a [LayoutBox],
    content: &'a dyn Fn(usize, T) -> T,
    containing: Vec<Option<usize>>,
    own: Vec<T>,
    inner: Vec<T>,
}

impl<T: Copy> Walk<'_, T> {
    fn visit(&mut self, index: usize, parents: T) {
        let own = self.containing[index].map_or(parents, |block| self.inner[block]);
        self.own[index] = own;
        self.inner[index] = (self.content)(index, own);
        for &child in &self.boxes[index].children {
            self.visit(child, self.inner[index]);
        }
    }
}
