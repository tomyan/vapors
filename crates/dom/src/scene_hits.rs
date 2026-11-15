//! Where a 3D scene drawn on a `gpu-surface` is hit
//! (docs/plans/css-for-3d.md, C4): the scene component gives the tree a
//! [`HitSource`] through [`Dom::watch_hits`](crate::Dom::watch_hits),
//! and the tree asks it whenever the pointer's hit test lands on the
//! surface. The anchor it answers becomes the target: `:hover` and
//! `:active` go on it and its ancestors, and pointer events go to it and
//! bubble through the scene's anchors to the scene anchor and on through
//! that anchor's HTML ancestors.

use crate::NodeId;

/// A point over a scene's `gpu-surface`, asked of its [`HitSource`].
pub struct HitAsk<'a> {
    /// The point, in CSS pixels from the top left of the surface's
    /// content box, within `0..=size`: a point over the surface's border
    /// is the surface's hit, and the source is not asked.
    pub point: [f32; 2],
    /// The content box's width and height, in CSS pixels.
    pub size: [f32; 2],
    /// Whether an anchor takes the pointer: its computed
    /// `pointer-events` is `auto` and its `visibility` is `visible`. A
    /// source passes over the nodes whose anchors do not; an answer that
    /// does not take it is refused. Asked about an anchor added since the
    /// last update, it answers `false` and the tree discards the answer:
    /// the surface is the target, and the hover is tested again after the
    /// next update. Asked about anything but an anchor in the scene, it
    /// answers `false` and the next update is refused.
    pub takes: &'a dyn Fn(NodeId) -> bool,
}

/// Answers the anchor under an [`HitAsk`]'s point: `Ok(None)` over no
/// node, so the surface itself is the target. The anchor must be in the
/// watched scene anchor's subtree and take the pointer. An `Err` is
/// refused: the surface is the target for that hit, and the next update
/// fails naming the scene anchor and the message (once per message). It
/// must not call the tree.
pub type HitSource = Box<dyn Fn(HitAsk<'_>) -> Result<Option<NodeId>, String>>;
