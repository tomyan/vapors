//! The clip each box paints and takes the pointer under (F2c S8): the
//! padding boxes of the `overflow: hidden` or `clip` boxes it is cut
//! off by, intersected. A box is cut off by what cuts off its
//! containing block's content: its parent's for a box in flow, the box
//! it is placed against for an absolute one, and nothing for a fixed
//! one, whose containing block is the viewport (box 0, the root, which
//! never clips).

use crate::layout::{BoxRect, LayoutBox, containing_blocks};

/// Each box's clip, CSS pixels, `None` for none; `clipping` says
/// whether a box cuts off its own content.
pub(crate) fn clips(
    boxes: &[LayoutBox],
    rects: &[BoxRect],
    clipping: &dyn Fn(usize) -> bool,
) -> Vec<Option<BoxRect>> {
    let mut walk = Walk {
        boxes,
        rects,
        clipping,
        containing: containing_blocks(boxes),
        own: vec![None; boxes.len()],
        content: vec![None; boxes.len()],
    };
    walk.visit(0, None);
    walk.own
}

struct Walk<'a> {
    boxes: &'a [LayoutBox],
    rects: &'a [BoxRect],
    clipping: &'a dyn Fn(usize) -> bool,
    containing: Vec<Option<usize>>,
    /// The clip each box is painted under.
    own: Vec<Option<BoxRect>>,
    /// The clip each box's content is painted under.
    content: Vec<Option<BoxRect>>,
}

impl Walk<'_> {
    /// Ancestors, and so containing blocks, are visited first.
    fn visit(&mut self, index: usize, parents: Option<BoxRect>) {
        let own = self.containing[index].map_or(parents, |block| self.content[block]);
        self.own[index] = own;
        self.content[index] = if (self.clipping)(index) {
            Some(within(
                own,
                padding_box(&self.boxes[index], self.rects[index]),
            ))
        } else {
            own
        };
        for &child in &self.boxes[index].children {
            self.visit(child, self.content[index]);
        }
    }
}

/// `rect` cut to `clip`; an empty rect where they do not meet.
pub(crate) fn within(clip: Option<BoxRect>, rect: BoxRect) -> BoxRect {
    let Some(clip) = clip else {
        return rect;
    };
    let x = rect.x.max(clip.x);
    let y = rect.y.max(clip.y);
    BoxRect {
        x,
        y,
        width: ((rect.x + rect.width).min(clip.x + clip.width) - x).max(0.0),
        height: ((rect.y + rect.height).min(clip.y + clip.height) - y).max(0.0),
    }
}

fn padding_box(layout_box: &LayoutBox, border_box: BoxRect) -> BoxRect {
    let border = layout_box.layout.border;
    BoxRect {
        x: border_box.x + border.left,
        y: border_box.y + border.top,
        width: (border_box.width - border.left - border.right).max(0.0),
        height: (border_box.height - border.top - border.bottom).max(0.0),
    }
}
