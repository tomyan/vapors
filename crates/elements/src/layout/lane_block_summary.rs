//! A block lane's summary of a run of children (`docs/plans/splice-
//! layout.md`, section 2.1): its top and bottom collapsible margin sets
//! and the height from its first child's border-box top to its last
//! child's border-box bottom, with the margins between them collapsed
//! as Taffy 0.15.1's block layout collapses them
//! (`perform_final_layout_on_in_flow_children`). A child whose margins
//! collapse through it is declined until PR 3b.

use taffy::CollapsibleMarginSet;

use super::lane_bounds::Bounds;
use super::lane_tree::Summary;
pub(crate) use crate::lane_decline::LaneDecline;

/// Collapsible margins: Taffy's own set (the largest positive and the
/// most negative, f32), so a child's set from its answer is kept as it
/// is; only the heights are summed in f64.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Margins(CollapsibleMarginSet);

impl Default for Margins {
    fn default() -> Self {
        Self(CollapsibleMarginSet::ZERO)
    }
}

impl Margins {
    /// One margin's set.
    #[cfg(test)]
    #[allow(clippy::cast_possible_truncation)] // a margin in CSS px, well within f32
    pub(crate) fn of(margin: f64) -> Self {
        Self(CollapsibleMarginSet::from_margin(margin as f32))
    }

    /// A set as Taffy made it.
    pub(crate) fn set(set: CollapsibleMarginSet) -> Self {
        Self(set)
    }

    /// The two sets collapsed into one (Taffy's
    /// `CollapsibleMarginSet::collapse_with_set`).
    pub(crate) fn collapse(self, other: Self) -> Self {
        Self(self.0.collapse_with_set(other.0))
    }

    /// The collapsed margin's size.
    pub(crate) fn resolve(self) -> f64 {
        f64::from(self.0.resolve())
    }

    /// The set, for a container's answer.
    pub(crate) fn as_set(self) -> CollapsibleMarginSet {
        self.0
    }
}

/// One child as block layout laid it out: its collapsible margin sets
/// are its own margin joined with the sets its descendants collapsed
/// through to its edges (taffy `block.rs`, `perform_final_layout_on_
/// in_flow_children`: `item_layout.top_margin.collapse_with_margin(
/// item_margin.top)`), as `SizeAnswer` records them (`asked.rs`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct BlockChild {
    pub(crate) top: Margins,
    pub(crate) bottom: Margins,
    pub(crate) height: f64,
    pub(crate) collapses_through: bool,
    pub(crate) bounds: Bounds,
}

/// A run of a block lane's children; `None` for no children.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct BlockSummary(Option<BlockRun>);

#[derive(Clone, Copy, Debug, PartialEq)]
struct BlockRun {
    top: Margins,
    inner: f64,
    bottom: Margins,
    bounds: Bounds,
}

impl BlockSummary {
    /// `child`'s summary.
    ///
    /// # Errors
    /// The child collapses its margins through it.
    pub(crate) fn leaf(child: BlockChild) -> Result<Self, LaneDecline> {
        if child.collapses_through {
            return Err(LaneDecline::CollapsesThrough);
        }
        Ok(Self(Some(BlockRun {
            top: child.top,
            inner: child.height,
            bottom: child.bottom,
            bounds: child.bounds,
        })))
    }

    /// The run's height with its first top and last bottom margins.
    #[cfg(test)]
    pub(crate) fn outer(&self) -> f64 {
        self.0.map_or(0.0, |run| {
            run.top.resolve() + run.inner + run.bottom.resolve()
        })
    }

    /// The run's height from its first child's border-box top to its
    /// last child's border-box bottom; 0 with no children.
    pub(crate) fn inner(&self) -> f64 {
        self.0.map_or(0.0, |run| run.inner)
    }

    /// Whether the run has no children.
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    /// The first child's top set: the container's own collapsible top
    /// margin when it collapses with its children (PR 3, plan 10.4),
    /// and then every `start` is less this set's `resolve()`.
    pub(crate) fn top(&self) -> Margins {
        self.0.map_or(Margins::default(), |run| run.top)
    }

    /// The last child's bottom set, the container's collapsible bottom
    /// margin when it collapses with its children (PR 3).
    pub(crate) fn bottom(&self) -> Margins {
        self.0.map_or(Margins::default(), |run| run.bottom)
    }
}

impl Summary for BlockSummary {
    fn combine(&self, right: &Self, _gap: f64) -> Self {
        let (Some(left), Some(right)) = (self.0, right.0) else {
            return Self(self.0.or(right.0));
        };
        let offset = left.inner + left.bottom.collapse(right.top).resolve();
        Self(Some(BlockRun {
            top: left.top,
            inner: offset + right.inner,
            bottom: right.bottom,
            bounds: left.bounds.then(offset, &right.bounds),
        }))
    }

    fn start_of(left: &Self, run: &Self, _gap: f64) -> f64 {
        let top = run.0.map_or(Margins::default(), |run| run.top);
        match left.0 {
            None => top.resolve(),
            Some(left) => left.top.resolve() + left.inner + left.bottom.collapse(top).resolve(),
        }
    }

    fn bounds(&self) -> Bounds {
        self.0.map_or(Bounds::EMPTY, |run| run.bounds)
    }
}

#[cfg(test)]
#[path = "lane_block_summary_tests.rs"]
mod tests;
