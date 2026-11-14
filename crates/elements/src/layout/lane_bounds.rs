//! What a run of a lane's children reaches, folded the same way for a
//! block and a flex lane (`docs/plans/splice-layout.md`, section 10.2):
//! offsets from the run's first child's border-box start along the
//! axis, so a run moved by an offset moves them all; and across the
//! axis, in the container, where an offset moves nothing.

/// A run's lowest and highest offsets, its reach across the axis, its
/// first baseline and its cross-axis contributions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Bounds {
    /// The least offset a child's border box or overflow starts at:
    /// below 0 with negative margins or overflow above.
    pub(crate) low: f64,
    /// The greatest offset a child's border box or overflow ends at.
    pub(crate) reach: f64,
    /// The least offset a child's border box or overflow ends at: with
    /// the run's start, whether some child ends before a scroll
    /// container's origin (`lane_block_answer.rs`).
    pub(crate) end: f64,
    /// The least start and the greatest end a child's border box or
    /// overflow reaches across the axis, in the container.
    pub(crate) across: [f64; 2],
    /// The least end a child reaches across the axis: whether some
    /// child ends before a scroll container's origin.
    pub(crate) across_end: f64,
    /// The first child's first baseline that has one.
    pub(crate) baseline: Option<f64>,
    /// The largest min-content and max-content cross-axis contribution.
    pub(crate) cross: [f64; 2],
}

impl Bounds {
    /// No children: the fold's identity, never a leaf's bounds. Its
    /// extents are the empty interval, so `then` with it on either
    /// side gives the other side; a container's answer clamps the
    /// folded extents to its origin (Taffy's union with `Rect::ZERO`).
    pub(crate) const EMPTY: Self = Self {
        low: f64::INFINITY,
        reach: f64::NEG_INFINITY,
        end: f64::INFINITY,
        across: [f64::INFINITY, f64::NEG_INFINITY],
        across_end: f64::INFINITY,
        baseline: None,
        cross: [0.0; 2],
    };

    /// A leaf reaching from `low` to `reach` along the axis and nothing
    /// across it: enough for a tree test.
    #[cfg(test)]
    pub(crate) fn span(low: f64, reach: f64) -> Self {
        Self {
            low,
            reach,
            end: reach,
            across: [0.0; 2],
            across_end: 0.0,
            baseline: None,
            cross: [0.0; 2],
        }
    }

    /// `self` followed by `right`, whose first child starts `offset`
    /// after `self`'s first child.
    pub(crate) fn then(&self, offset: f64, right: &Self) -> Self {
        Self {
            low: self.low.min(offset + right.low),
            reach: self.reach.max(offset + right.reach),
            end: self.end.min(offset + right.end),
            across: [
                self.across[0].min(right.across[0]),
                self.across[1].max(right.across[1]),
            ],
            across_end: self.across_end.min(right.across_end),
            baseline: self.baseline.or(right.baseline.map(|b| offset + b)),
            cross: [
                self.cross[0].max(right.cross[0]),
                self.cross[1].max(right.cross[1]),
            ],
        }
    }
}

/// Where a child's extent ends, for the earliest end of a run: an empty
/// extent (`[inf, -inf]`) adds nothing, as Taffy leaves an empty
/// contribution out before asking whether it is before the origin
/// (`scrollable_overflow.rs` 42-47).
pub(crate) fn earliest_end(extent: [f64; 2]) -> f64 {
    if extent[0] > extent[1] {
        f64::INFINITY
    } else {
        extent[1]
    }
}

#[cfg(test)]
#[path = "lane_bounds_tests.rs"]
mod tests;
