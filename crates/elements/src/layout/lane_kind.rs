//! What a block lane and a flex lane do differently
//! (`docs/plans/splice-layout.md`, section 10.1): the summary their
//! tree folds, the container's settled inputs, laying one child out,
//! where the first child's start is measured from, and the container's
//! answer. `lane_fill.rs` and `lane_layout.rs` do the rest for both.

use std::fmt::Debug;

use rustc_hash::FxHashMap;
use taffy::{Layout, LayoutOutput};

use super::lane::Lane;
use super::lane_set::Lanes;
use super::lane_tree::Summary;
use super::tree::BoxTree;
use crate::lane_decline::LaneDecline;

/// A layout algorithm a lane lays a container's children out with.
pub(crate) trait LaneKind: Sized + Debug {
    type Summary: Summary + Copy + Default + Debug;
    /// The container's inputs, settled for one ask.
    type Container;
    /// What every child's leaf depends on but the height bases.
    type Share: Clone + Debug + PartialEq;
    /// The height bases, which change between a flex parent's size
    /// query and its layout; only children whose styles read them
    /// (`lane_share::reads_heights`) are laid out again for them.
    type Heights: Copy + Debug + PartialEq;

    fn share(c: &Self::Container) -> Self::Share;
    fn heights(c: &Self::Container) -> Self::Heights;
    /// The gap between neighbouring children along the axis.
    fn gap(c: &Self::Container) -> f64;
    /// Whether this ask lays the children out, so their starts are
    /// written.
    fn writes(c: &Self::Container) -> bool;
    /// Where the starts are measured from in the container's border
    /// box, given the run.
    fn top(c: &Self::Container, run: &Self::Summary) -> f64;
    /// The child in `slot` laid out: its leaf, and its layout with only
    /// its relative offset along the axis.
    ///
    /// # Errors
    /// The child is not one the lane lays out.
    fn lay_out_child(
        tree: &mut BoxTree<'_>,
        slot: usize,
        c: &Self::Container,
    ) -> Result<(Self::Summary, Layout), LaneDecline>;
    /// The container's answer from its children's fold.
    ///
    /// # Errors
    /// The fold shows a case the lane does not lay out.
    fn answer(c: &Self::Container, run: &Self::Summary) -> Result<LayoutOutput, LaneDecline>;
    /// Whether a child styled `style` reads the height bases, so its
    /// leaf is made again when they change.
    fn reads_heights(style: &taffy::Style) -> bool;
    /// This kind's lanes.
    fn lanes(lanes: &mut Lanes) -> &mut FxHashMap<usize, Lane<Self>>;
}
