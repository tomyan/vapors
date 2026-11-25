//! Scroll offsets (F2c S9): one per scroll container (`overflow:
//! scroll` or `auto`), in CSS pixels, kept between updates. Each update
//! clamps them to the content and drops those of elements that no
//! longer scroll; paint applies them rounded to whole device pixels.

use crate::node_map::NodeMap;

use vapors_dom::NodeId;

use crate::layout::{Content, Lanes, LayoutBox};
use crate::scroll_reveal::Revealed;

/// The offsets, by scroll container, and the containers scrolled
/// since the last update.
#[derive(Clone, Debug, Default)]
pub(crate) struct Offsets {
    by_node: NodeMap<Offset>,
    scrolled: Vec<NodeId>,
}

/// One container's offset and how far it can go, and both as last
/// settled: a scroll moves `at` before the update settles it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Offset {
    at: [f32; 2],
    max: [f32; 2],
    settled: [[f32; 2]; 2],
    /// How opaque its own thumbs are drawn, 0 to 255 (`thumb.rs`), and
    /// as last settled.
    alpha: u8,
    settled_alpha: u8,
    /// A text field's caret, as its offset last followed it
    /// (`scroll_reveal.rs`).
    revealed: Revealed,
}

impl Offsets {
    /// The offset of `node`, if it is a scroll container.
    pub(crate) fn get(&self, node: NodeId) -> Option<[f32; 2]> {
        self.by_node.get(&node).map(|offset| offset.at)
    }

    /// Each scroll container and its offset, in no order.
    pub(crate) fn each(&self) -> impl Iterator<Item = (NodeId, [f32; 2])> + '_ {
        self.by_node.iter().map(|(node, offset)| (*node, offset.at))
    }

    /// The offset of `node` and how far it can go, if it scrolls.
    pub(crate) fn state(&self, node: NodeId) -> Option<([f32; 2], [f32; 2])> {
        self.by_node
            .get(&node)
            .map(|offset| (offset.at, offset.max))
    }

    /// How opaque `node`'s own thumbs are drawn, 0 (hidden) to 255.
    pub(crate) fn alpha(&self, node: NodeId) -> u8 {
        self.by_node.get(&node).map_or(0, |offset| offset.alpha)
    }

    /// Draws `node`'s own thumbs at `alpha` from the next update, which
    /// settles and repaints them as a scroll does; whether it changed
    /// (false when `node` is not a scroll container).
    pub(crate) fn set_alpha(&mut self, node: NodeId, alpha: u8) -> bool {
        let Some(offset) = self.by_node.get_mut(&node).filter(|o| o.alpha != alpha) else {
            return false;
        };
        offset.alpha = alpha;
        self.scrolled.push(node);
        true
    }

    /// Clamps the offsets to the laid-out `boxes`, where `scrolls` says
    /// which box is a scroll container, and returns each box's offset
    /// in CSS pixels rounded to whole device pixels at `scale`.
    pub(crate) fn settle(
        &mut self,
        (boxes, lanes): (&[LayoutBox], &Lanes),
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
                let offset = self.clamped(node, (boxes, lanes), index);
                kept.insert(node, offset);
                applied(offset.at, scale)
            })
            .collect();
        self.by_node = kept;
        self.scrolled.clear();
        applied
    }

    /// Clamps the offset of `node`, the scroll container in box `index`
    /// of `boxes`, and returns it as `settle` does; and whether the
    /// offset or the range, unrounded, or its thumbs' alpha differs from
    /// when it was last settled (exact), which repaints its thumbs.
    pub(crate) fn settle_box(
        &mut self,
        node: NodeId,
        (boxes, lanes, index): (&[LayoutBox], &Lanes, usize),
        scale: f32,
    ) -> ([f32; 2], bool) {
        let offset = self.clamped(node, (boxes, lanes), index);
        self.settled_to(node, offset, scale)
    }

    /// As [`settle_box`](Self::settle_box) for `node`, a container that
    /// only scrolled since it was last settled: clamped to the range it
    /// kept, which only a new layout changes (4g), not measured again.
    ///
    /// # Panics
    /// When `node` kept no offset: a container is scrolled only when it
    /// holds one (`scroll`), so the walk marked a box `Scrolled` that was
    /// never settled.
    pub(crate) fn settle_scrolled(&mut self, node: NodeId, scale: f32) -> ([f32; 2], bool) {
        let max = self.by_node.get(&node).map(|kept| kept.max);
        let max = max.unwrap_or_else(|| {
            panic!(
                "scroll container {node:?} is marked as only scrolled but kept no offset, so \
                 no range to clamp to: a box marked Scrolled was never settled \
                 (placement_marks.rs); reproduce with `cargo xtask test`"
            )
        });
        let kept = &self.by_node[&node];
        let offset = self.clamped_to(node, max, (kept.at, kept.revealed));
        self.settled_to(node, offset, scale)
    }

    /// `offset` kept for `node`; it as applied at `scale`, and whether it
    /// or its thumbs' alpha differs from when last settled (exact).
    fn settled_to(&mut self, node: NodeId, offset: Offset, scale: f32) -> ([f32; 2], bool) {
        let was = self
            .by_node
            .insert(node, offset)
            .map(|o| (bits(o.settled), o.settled_alpha));
        let now = (bits(offset.settled), offset.settled_alpha);
        (applied(offset.at, scale), was != Some(now))
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

    /// `node`'s offset, a text field's moved to its caret
    /// (`scroll_reveal.rs`), clamped to its range as laid out now.
    fn clamped(&self, node: NodeId, laid: (&[LayoutBox], &Lanes), index: usize) -> Offset {
        let kept = self.by_node.get(&node);
        let at = kept.map_or([0.0; 2], |kept| kept.at);
        let last = kept.and_then(|kept| kept.revealed);
        let moved = crate::scroll_reveal::revealed(laid.0, index, at, last);
        self.clamped_to(node, range(laid, index), moved)
    }

    /// `node`'s offset `at` clamped to `max`, its range, kept with what
    /// it was `revealed` for.
    fn clamped_to(
        &self,
        node: NodeId,
        max: [f32; 2],
        (at, revealed): ([f32; 2], Revealed),
    ) -> Offset {
        let at = [at[0].clamp(0.0, max[0]), at[1].clamp(0.0, max[1])];
        let alpha = self.alpha(node);
        Offset {
            at,
            max,
            settled: [at, max],
            alpha,
            settled_alpha: alpha,
            revealed,
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

/// `pair`'s bits, for an exact comparison.
fn bits(pair: [[f32; 2]; 2]) -> [[u32; 2]; 2] {
    pair.map(|v| v.map(f32::to_bits))
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
/// its padding box, on the axes that scroll (`auto`, `scroll`; a
/// `hidden` or `clip` axis has none). Taffy 0.15's scrollable overflow leaves out the end
/// margins of the in-flow children and the end padding, which Chromium
/// counts, so the larger of the two is taken. A text field's on x is
/// its text's and caret's past its content box (`scroll_reveal.rs`).
fn range((boxes, lanes): (&[LayoutBox], &Lanes), index: usize) -> [f32; 2] {
    let layout = &boxes[index].layout;
    let (border, padding) = (layout.border, layout.padding);
    let end = content_end((boxes, lanes), index);
    let inner = [
        layout.size.width - border.left - border.right,
        layout.size.height - border.top - border.bottom,
    ];
    let reach = [
        layout.scroll_width().max(end[0] + padding.right - inner[0]),
        layout
            .scroll_height()
            .max(end[1] + padding.bottom - inner[1]),
    ];
    let reach = if boxes[index].text_field {
        [crate::scroll_reveal::field_reach(boxes, index), reach[1]]
    } else {
        reach
    };
    let overflow = boxes[index].style.overflow;
    let scrolls = |axis: taffy::Overflow| axis == taffy::Overflow::Scroll;
    [
        if scrolls(overflow.x) { reach[0] } else { 0.0 },
        if scrolls(overflow.y) { reach[1] } else { 0.0 },
    ]
}

/// How far box `index`'s in-flow children reach, margins included, from
/// its padding box's corner, each where Taffy puts it.
fn content_end((boxes, lanes): (&[LayoutBox], &Lanes), index: usize) -> [f32; 2] {
    let border = boxes[index].layout.border;
    let in_flow = boxes[index].children.iter().copied();
    in_flow
        .filter(|&child| boxes[child].style.position != taffy::Position::Absolute)
        .map(|child| {
            (
                lanes.taffy_location(child, &boxes[child].layout),
                boxes[child].layout,
            )
        })
        .fold([0.0_f32, 0.0_f32], |end, (location, at)| {
            [
                end[0].max(location.x + at.size.width + at.margin.right - border.left),
                end[1].max(location.y + at.size.height + at.margin.bottom - border.top),
            ]
        })
}

#[cfg(test)]
#[path = "scroll_tests.rs"]
mod tests;
