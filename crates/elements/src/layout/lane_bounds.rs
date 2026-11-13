//! What a run of a lane's children reaches, folded the same way for a
//! block and a flex lane (`docs/plans/splice-layout.md`, section 10.2):
//! offsets from the run's first child's border-box start along the
//! axis, so a run moved by an offset moves them all.

/// A run's lowest and highest offsets, first baseline and cross-axis
/// contributions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Bounds {
    /// The least offset a child's border box or overflow starts at; 0,
    /// or below with negative margins.
    pub(crate) low: f64,
    /// The greatest offset a child's border box or overflow ends at.
    pub(crate) reach: f64,
    /// The first child's first baseline that has one.
    pub(crate) baseline: Option<f64>,
    /// The largest min-content and max-content cross-axis contribution.
    pub(crate) cross: [f64; 2],
}

impl Bounds {
    /// No children: the fold's empty value, never a leaf's. As the
    /// left side of `then` it clamps `low` and `reach` to 0, so it is
    /// not an identity: a leaf names its own `low` and `reach`.
    pub(crate) const EMPTY: Self = Self {
        low: 0.0,
        reach: 0.0,
        baseline: None,
        cross: [0.0; 2],
    };

    /// `self` followed by `right`, whose first child starts `offset`
    /// after `self`'s first child.
    pub(crate) fn then(&self, offset: f64, right: &Self) -> Self {
        Self {
            low: self.low.min(offset + right.low),
            reach: self.reach.max(offset + right.reach),
            baseline: self.baseline.or(right.baseline.map(|b| offset + b)),
            cross: [
                self.cross[0].max(right.cross[0]),
                self.cross[1].max(right.cross[1]),
            ],
        }
    }
}

#[cfg(test)]
#[path = "lane_bounds_tests.rs"]
mod tests;
