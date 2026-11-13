//! A block lane's summary of a run of children (`docs/plans/splice-
//! layout.md`, section 2.1): its top and bottom collapsible margin sets
//! and the height from its first child's border-box top to its last
//! child's border-box bottom, with the margins between them collapsed
//! as Taffy 0.15.1's block layout collapses them
//! (`perform_final_layout_on_in_flow_children`). A child whose margins
//! collapse through it is declined until PR 3b.

use super::lane_bounds::Bounds;
use super::lane_tree::Summary;

/// Collapsible margins: the largest positive and the most negative.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Margins {
    pub(crate) positive: f64,
    pub(crate) negative: f64,
}

impl Margins {
    /// One margin's set.
    pub(crate) fn of(margin: f64) -> Self {
        Self {
            positive: margin.max(0.0),
            negative: margin.min(0.0),
        }
    }

    /// The two sets collapsed into one.
    fn collapse(self, other: Self) -> Self {
        Self {
            positive: self.positive.max(other.positive),
            negative: self.negative.min(other.negative),
        }
    }

    /// The collapsed margin's size.
    pub(crate) fn resolve(self) -> f64 {
        self.positive + self.negative
    }
}

/// Why a child cannot be a lane's leaf.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LaneDecline {
    /// Its margins collapse through it (empty, no padding or border):
    /// the join is PR 3b's.
    CollapsesThrough,
}

/// One child as block layout laid it out.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BlockChild {
    pub(crate) margins: [f64; 2],
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
            top: Margins::of(child.margins[0]),
            inner: child.height,
            bottom: Margins::of(child.margins[1]),
            bounds: child.bounds,
        })))
    }

    /// The run's height with its first top and last bottom margins.
    pub(crate) fn outer(&self) -> f64 {
        self.0.map_or(0.0, |run| {
            run.top.resolve() + run.inner + run.bottom.resolve()
        })
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
