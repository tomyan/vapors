//! A flex lane's summary of a run of items (`docs/plans/splice-
//! layout.md`, section 10.2): margins never collapse, so a run is the
//! sum of its items' margin-box sizes plus one gap between neighbours,
//! and the two sums Taffy 0.15.1 sizes a flex container's auto main
//! size from (`determine_container_main_size`, `compute/flexbox.rs`:
//! the definite-space branch at 1420-1560, the min- and max-content
//! branch at 1700-1715).

use super::lane_bounds::Bounds;
use super::lane_tree::Summary;

/// One item as flex layout laid it out.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FlexItem {
    pub(crate) margins: [f64; 2],
    pub(crate) size: f64,
    /// `max(flex_basis, min_size)` plus margins, floored at padding
    /// and border: its share of a container sized under definite space.
    pub(crate) basis: f64,
    /// Its share of a container sized under min- or max-content space:
    /// `flex_basis + flex_contribution + margin` (`flexbox.rs:1700-1715`),
    /// where for `flex-grow: 0` the contribution is the item's content
    /// contribution above its basis (`content_flex_fraction`,
    /// 1660-1670), and 0 when its scaled shrink factor is 0 (an inner
    /// flex basis of 0 under a clamped contribution): then this is the
    /// basis plus margins, not the content contribution. PR 3f computes
    /// it as Taffy does, per item.
    pub(crate) content: f64,
    pub(crate) bounds: Bounds,
}

/// A run of a flex lane's items.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct FlexSummary(Option<FlexRun>);

#[derive(Clone, Copy, Debug, PartialEq)]
struct FlexRun {
    /// The items' margin-box sizes, summed.
    outer: f64,
    count: u32,
    /// The first item's start margin and the last item's end margin.
    leading: f64,
    trailing: f64,
    basis: f64,
    content: f64,
    bounds: Bounds,
}

impl FlexSummary {
    /// `item`'s summary.
    pub(crate) fn leaf(item: FlexItem) -> Self {
        Self(Some(FlexRun {
            outer: item.margins[0] + item.size + item.margins[1],
            count: 1,
            leading: item.margins[0],
            trailing: item.margins[1],
            basis: item.basis,
            content: item.content,
            bounds: item.bounds,
        }))
    }

    /// The run's main size: its items' margin boxes and the gaps
    /// between them.
    pub(crate) fn outer(&self, gap: f64) -> f64 {
        self.0
            .map_or(0.0, |run| run.outer + gaps(gap, run.count - 1))
    }

    /// The base-size and content sums with their gaps.
    pub(crate) fn contributions(&self, gap: f64) -> [f64; 2] {
        self.0.map_or([0.0; 2], |run| {
            let gaps = gaps(gap, run.count - 1);
            [run.basis + gaps, run.content + gaps]
        })
    }
}

impl Summary for FlexSummary {
    fn combine(&self, right: &Self, gap: f64) -> Self {
        let (Some(left), Some(right)) = (self.0, right.0) else {
            return Self(self.0.or(right.0));
        };
        let between = gaps(gap, left.count);
        let offset = left.outer - left.leading + between + right.leading;
        Self(Some(FlexRun {
            outer: left.outer + right.outer,
            count: left.count + right.count,
            leading: left.leading,
            trailing: right.trailing,
            basis: left.basis + right.basis,
            content: left.content + right.content,
            bounds: left.bounds.then(offset, &right.bounds),
        }))
    }

    fn start_of(left: &Self, run: &Self, gap: f64) -> f64 {
        let leading = run.0.map_or(0.0, |run| run.leading);
        let before = left
            .0
            .map_or(0.0, |left| left.outer + gaps(gap, left.count));
        before + leading
    }

    fn bounds(&self) -> Bounds {
        self.0.map_or(Bounds::EMPTY, |run| run.bounds)
    }
}

/// `count` gaps.
fn gaps(gap: f64, count: u32) -> f64 {
    gap * f64::from(count)
}

#[cfg(test)]
#[path = "lane_flex_summary_tests.rs"]
mod tests;
