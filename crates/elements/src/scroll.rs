//! Scroll offsets (F2c S9): one per scroll container (`overflow:
//! scroll` or `auto`), in CSS pixels, kept between updates. Each update
//! clamps them to the content and drops those of elements that no
//! longer scroll; paint applies them rounded to whole device pixels.

use crate::node_map::NodeMap;

use vapors_dom::NodeId;

use crate::layout::{Content, LayoutBox};

/// The offsets, by scroll container, and the containers scrolled
/// since the last update.
#[derive(Clone, Debug, Default)]
pub(crate) struct Offsets {
    by_node: NodeMap<Offset>,
    scrolled: Vec<NodeId>,
}

/// One container's offset and how far it can go.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Offset {
    at: [f32; 2],
    max: [f32; 2],
}

impl Offsets {
    /// The offset of `node`, if it is a scroll container.
    pub(crate) fn get(&self, node: NodeId) -> Option<[f32; 2]> {
        self.by_node.get(&node).map(|offset| offset.at)
    }

    /// The offset of `node` and how far it can go, if it scrolls.
    pub(crate) fn state(&self, node: NodeId) -> Option<([f32; 2], [f32; 2])> {
        self.by_node
            .get(&node)
            .map(|offset| (offset.at, offset.max))
    }

    /// Clamps the offsets to the laid-out `boxes`, where `scrolls` says
    /// which box is a scroll container, and returns each box's offset
    /// in CSS pixels rounded to whole device pixels at `scale`.
    pub(crate) fn settle(
        &mut self,
        boxes: &[LayoutBox],
        scrolls: &dyn Fn(usize) -> bool,
        scale: f32,
    ) -> Vec<[f32; 2]> {
        let mut kept = NodeMap::default();
        let applied = (0..boxes.len())
            .map(|index| {
                let Content::Element(node) = boxes[index].content else {
                    return [0.0, 0.0];
                };
                if !scrolls(index) {
                    return [0.0, 0.0];
                }
                let offset = self.clamped(node, boxes, index);
                kept.insert(node, offset);
                applied(offset.at, scale)
            })
            .collect();
        self.by_node = kept;
        self.scrolled.clear();
        applied
    }

    /// Clamps the offset of `node`, the scroll container in box `index`
    /// of `boxes`, and returns it as `settle` does.
    pub(crate) fn settle_box(
        &mut self,
        node: NodeId,
        (boxes, index): (&[LayoutBox], usize),
        scale: f32,
    ) -> [f32; 2] {
        let offset = self.clamped(node, boxes, index);
        self.by_node.insert(node, offset);
        applied(offset.at, scale)
    }

    /// Drops the offset of `node`, which no longer scrolls.
    pub(crate) fn forget(&mut self, node: NodeId) {
        self.by_node.remove(&node);
    }

    /// Keeps only the offsets of `nodes`.
    pub(crate) fn keep_only(&mut self, nodes: &crate::node_map::NodeSet) {
        self.by_node.retain(|node, _| nodes.contains(node));
    }

    /// The containers scrolled since the last call.
    pub(crate) fn take_scrolled(&mut self) -> Vec<NodeId> {
        std::mem::take(&mut self.scrolled)
    }

    /// Whether these offsets are `other`'s, scrolls since aside.
    pub(crate) fn same_offsets(&self, other: &Self) -> bool {
        self.by_node == other.by_node
    }

    fn clamped(&self, node: NodeId, boxes: &[LayoutBox], index: usize) -> Offset {
        let max = range(boxes, index);
        let at = self.get(node).unwrap_or_default();
        Offset {
            at: [at[0].clamp(0.0, max[0]), at[1].clamp(0.0, max[1])],
            max,
        }
    }

    /// Moves `node` by `delta`, within its range; whether it moved.
    pub(crate) fn scroll(&mut self, node: NodeId, delta: [f32; 2]) -> bool {
        let Some(offset) = self.by_node.get_mut(&node) else {
            return false;
        };
        let (moved, changed) = offset.moved_by(delta);
        offset.at = moved;
        if changed {
            self.scrolled.push(node);
        }
        changed
    }

    /// Whether `node` would move by `delta`.
    pub(crate) fn can_move(&self, node: NodeId, delta: [f32; 2]) -> bool {
        self.by_node
            .get(&node)
            .is_some_and(|offset| offset.moved_by(delta).1)
    }
}

impl Offset {
    /// Where `delta` takes this offset, within its range, and whether
    /// that is a move at all (exact: any change is one to show).
    fn moved_by(&self, delta: [f32; 2]) -> ([f32; 2], bool) {
        let moved = [0, 1].map(|axis| (self.at[axis] + delta[axis]).clamp(0.0, self.max[axis]));
        (moved, moved.map(f32::to_bits) != self.at.map(f32::to_bits))
    }
}

/// The shift a scroll container's offset `at` gives the content of its
/// frame at `scale`: device pixels, whole. The one answer to how far a
/// scroll moves its frame: paint's frames (`paint_partial.rs`), the
/// placement check (`placement_check_page.rs`) and, through
/// [`applied`], placement and the a11y mirror read it.
pub(crate) fn frame_shift(at: [f32; 2], scale: f32) -> [f32; 2] {
    at.map(|v| -(v * scale).round())
}

/// An offset `at` as paint applies it, in CSS pixels: its frame shift
/// ([`frame_shift`]) at `scale`, negated.
pub(crate) fn applied(at: [f32; 2], scale: f32) -> [f32; 2] {
    frame_shift(at, scale).map(|shift| -shift / scale)
}

/// How far box `index` scrolls on each axis: its content's extent past
/// its padding box. Taffy 0.15's scrollable overflow leaves out the end
/// margins of the in-flow children and the end padding, which Chromium
/// counts, so the larger of the two is taken.
fn range(boxes: &[LayoutBox], index: usize) -> [f32; 2] {
    let layout = &boxes[index].layout;
    let (border, padding) = (layout.border, layout.padding);
    let end = content_end(boxes, index);
    let inner = [
        layout.size.width - border.left - border.right,
        layout.size.height - border.top - border.bottom,
    ];
    [
        layout.scroll_width().max(end[0] + padding.right - inner[0]),
        layout
            .scroll_height()
            .max(end[1] + padding.bottom - inner[1]),
    ]
}

/// How far box `index`'s in-flow children reach, margins included, from
/// its padding box's corner.
fn content_end(boxes: &[LayoutBox], index: usize) -> [f32; 2] {
    let border = boxes[index].layout.border;
    let in_flow = boxes[index].children.iter().map(|&child| &boxes[child]);
    in_flow
        .filter(|child| child.style.position != taffy::Position::Absolute)
        .map(|child| child.layout)
        .fold([0.0_f32, 0.0_f32], |end, at| {
            [
                end[0].max(at.location.x + at.size.width + at.margin.right - border.left),
                end[1].max(at.location.y + at.size.height + at.margin.bottom - border.top),
            ]
        })
}
