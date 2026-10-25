//! Scroll offsets (F2c S9): one per scroll container (`overflow:
//! scroll` or `auto`), in CSS pixels, kept between updates. Each update
//! clamps them to the content and drops those of elements that no
//! longer scroll; paint applies them rounded to whole device pixels.

use std::collections::HashMap;

use vapors_dom::NodeId;

use crate::layout::{Content, LayoutBox};

/// The offsets, by scroll container.
#[derive(Debug, Default)]
pub(crate) struct Offsets {
    by_node: HashMap<NodeId, Offset>,
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

    /// Clamps the offsets to the laid-out `boxes`, where `scrolls` says
    /// which box is a scroll container, and returns each box's offset
    /// in CSS pixels rounded to whole device pixels at `scale`.
    pub(crate) fn settle(
        &mut self,
        boxes: &[LayoutBox],
        scrolls: &dyn Fn(usize) -> bool,
        scale: f32,
    ) -> Vec<[f32; 2]> {
        let mut kept = HashMap::new();
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
                offset.at.map(|v| (v * scale).round() / scale)
            })
            .collect();
        self.by_node = kept;
        applied
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

/// How far box `index` scrolls on each axis: its content's extent past
/// its padding box. Taffy 0.15's scrollable overflow leaves out the end
/// margins of the in-flow children and the end padding, which Chromium
/// counts, so the larger of the two is taken.
fn range(boxes: &[LayoutBox], index: usize) -> [f32; 2] {
    let layout = &boxes[index].layout;
    let (border, padding) = (layout.border, layout.padding);
    let mut end = [0.0_f32, 0.0_f32];
    for &child in &boxes[index].children {
        let child = &boxes[child];
        if child.style.position == taffy::Position::Absolute {
            continue;
        }
        let at = child.layout;
        end[0] = end[0].max(at.location.x + at.size.width + at.margin.right - border.left);
        end[1] = end[1].max(at.location.y + at.size.height + at.margin.bottom - border.top);
    }
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

/// Which scroll container a scroll over an element moves first, and
/// which one each container passes a scroll it cannot take to: by
/// containing block, as clips pass (`crate::chain`).
#[derive(Debug, Default)]
pub(crate) struct Chains {
    first: HashMap<NodeId, NodeId>,
    outer: HashMap<NodeId, NodeId>,
}

impl Chains {
    /// The chains of the laid-out `boxes`, where `scrolls` says which
    /// box is a scroll container.
    pub(crate) fn of(boxes: &[LayoutBox], scrolls: &dyn Fn(usize) -> bool) -> Self {
        let node = |index: usize| match boxes[index].content {
            Content::Element(node) => Some(node),
            Content::Text(_) => None,
        };
        let around = crate::chain::by_containing_block(boxes, None, &|index, own| {
            if scrolls(index) { node(index) } else { own }
        });
        let mut chains = Self::default();
        for (index, own) in around.into_iter().enumerate() {
            let Some(element) = node(index) else { continue };
            let first = if scrolls(index) { Some(element) } else { own };
            if let Some(first) = first {
                chains.first.insert(element, first);
            }
            if let (true, Some(outer)) = (scrolls(index), own) {
                chains.outer.insert(element, outer);
            }
        }
        chains
    }

    /// The scroll containers a scroll over `element` tries, innermost
    /// first.
    pub(crate) fn from(&self, element: NodeId) -> Vec<NodeId> {
        let mut found = Vec::new();
        let mut next = self.first.get(&element).copied();
        while let Some(container) = next {
            found.push(container);
            next = self.outer.get(&container).copied();
        }
        found
    }
}
