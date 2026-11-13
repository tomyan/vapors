//! A record's bounds and scroll (R6 PR 5a, `docs/plans/r6.md` "Element
//! mapping"): the border box from the border box of the node above it,
//! plus that node's scroll offset as applied, so a node's record does
//! not change when a container above it scrolls; and a scroll
//! container's offset and extent, which the mirror's element scrolls
//! by. A scroll container with no role is in the tree as `none`, so
//! its rows nest under its element and one scroll write moves them. A
//! node whose box is placed outside its parent node's box (hoisted: a
//! popover whose containing block is above the parent) does not move
//! with the parent, so its bounds are from the viewport, which keeps
//! them right when the parent moves or scrolls without it; a scroll of
//! a container whose frame its box is in reads it again
//! (`a11y_mirror_reach.rs`).

use vapors_a11y::{AxRect, AxScroll};
use vapors_dom::NodeId;

use crate::a11y_roles::Ids;
use crate::document::Document;
use crate::layout::BoxRect;
use crate::placement::NO_SLOT;
use crate::scroll::applied;

impl Document {
    /// `node`'s role in the mirror: its role in the tree, or `none` for
    /// a scroll container with none, which the mirror holds as the
    /// element its rows scroll in.
    pub(crate) fn ax_mirror_role(&self, node: NodeId, ids: &Ids) -> Option<String> {
        self.ax_role(node, ids)
            .or_else(|| self.scroll.state(node).map(|_| "none".to_owned()))
    }

    /// `node`'s `bounds` from the border box of `parent`, which the
    /// mirror's element has as its padding box, scrolled by `parent`'s
    /// applied offset; from the viewport for a top node or a hoisted
    /// one (the flag), whose box is not placed in the parent's.
    pub(crate) fn ax_relative(
        &self,
        node: NodeId,
        bounds: BoxRect,
        parent: Option<NodeId>,
    ) -> (AxRect, bool) {
        let hoisted = parent.is_some_and(|parent| !self.ax_placed_in(node, parent));
        let origin = match parent {
            Some(parent) if !hoisted => self.ax_content_origin(parent),
            _ => [0.0, 0.0],
        };
        let rect = AxRect {
            x: bounds.x - origin[0],
            y: bounds.y - origin[1],
            width: bounds.width,
            height: bounds.height,
        };
        (rect, hoisted)
    }

    /// Whether `node`'s box is placed in `parent`'s: the chain of boxes
    /// it is placed in (its containing block, for an out-of-flow box)
    /// passes `parent`'s, so it moves and scrolls with it.
    fn ax_placed_in(&self, node: NodeId, parent: NodeId) -> bool {
        self.boxes
            .slot_of(parent)
            .is_some_and(|target| self.ax_placed_under(node, &|slot| slot == target))
    }

    /// Whether the chain of boxes `node`'s box is placed in passes a
    /// slot `under` holds for; false for a node with no box.
    pub(crate) fn ax_placed_under(&self, node: NodeId, under: &dyn Fn(usize) -> bool) -> bool {
        let painted = self.painted.as_ref().expect("read from the last update");
        let up = &painted.from.placement.up;
        let Some(slot) = self.boxes.slot_of(node) else {
            return false;
        };
        let mut cursor = up[slot];
        while cursor != NO_SLOT {
            if under(cursor) {
                return true;
            }
            cursor = up[cursor];
        }
        false
    }

    /// Where `node`'s content starts: its border box's corner, less its
    /// scroll offset as applied.
    fn ax_content_origin(&self, node: NodeId) -> [f32; 2] {
        let painted = self.painted.as_ref().expect("read from the last update");
        // the node above is in the tree, so it has a box (a11y_mirror_record.rs)
        let rect = painted.border_box(node).expect("a node above has a box");
        let offset = self.ax_applied_offset(node);
        [rect.x - offset[0], rect.y - offset[1]]
    }

    /// The scroll of `node`, when it is a scroll container: its offset
    /// as applied, and the extent its element must scroll over, which
    /// is `bounds` (the element's padding box) plus how far it can go.
    pub(crate) fn ax_scroll(&self, node: NodeId, bounds: BoxRect) -> Option<AxScroll> {
        let (_, max) = self.scroll.state(node)?;
        Some(AxScroll {
            offset: self.ax_applied_offset(node),
            extent: [bounds.width + max[0], bounds.height + max[1]],
        })
    }

    /// `node`'s scroll offset as paint applied it: `[0, 0]` for a node
    /// that does not scroll.
    fn ax_applied_offset(&self, node: NodeId) -> [f32; 2] {
        let scale = self
            .painted
            .as_ref()
            .expect("read from the last update")
            .scale;
        self.scroll
            .get(node)
            .map_or([0.0, 0.0], |at| applied(at, scale))
    }
}
