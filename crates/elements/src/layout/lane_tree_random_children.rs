//! The random children of `lane_tree_random_tests.rs` and the
//! sequential passes they are checked against: Taffy 0.15.1's running
//! collapse of margin sets for block, sums and gaps for flex. Sizes are
//! multiples of a quarter, so every sum is exact.

use crate::layout::lane_block_summary::{BlockChild, BlockSummary, Margins};
use crate::layout::lane_bounds::Bounds;
use crate::layout::lane_flex_summary::{FlexItem, FlexSummary};

/// A child as the sequential pass reads it.
#[derive(Clone, Copy, Debug)]
pub(super) struct Child {
    /// Its own top and bottom margins.
    pub(super) margins: [f64; 2],
    /// A descendant's margin collapsed through to each edge, joined
    /// into the child's block set (review of #293, M1); 0 for most. A
    /// flex item's margins never collapse, so flex ignores it.
    pub(super) through: [f64; 2],
    pub(super) size: f64,
    /// Overflow above its border box, and below it.
    pub(super) overflow: [f64; 2],
}

/// A collapsible margin set in plain floats: the largest positive and
/// the most negative of its margins.
type Set = (f64, f64);

fn set(margins: &[f64]) -> Set {
    let join =
        |(positive, negative): Set, &margin: &f64| (positive.max(margin), negative.min(margin));
    margins.iter().fold((0.0, 0.0), join)
}

impl Child {
    fn top_set(self) -> Set {
        set(&[self.margins[0], self.through[0]])
    }

    fn bottom_set(self) -> Set {
        set(&[self.margins[1], self.through[1]])
    }

    pub(super) fn low(self) -> f64 {
        -self.overflow[0]
    }

    pub(super) fn reach(self) -> f64 {
        self.size + self.overflow[1]
    }
}

/// xorshift64, seeded: the same edits on every run; margins drawn in
/// quarters from `margins.0..margins.1`.
pub(super) struct Random {
    state: u64,
    margins: (i32, i32),
}

/// Margins in -8..24.
pub(super) const MIXED: (i32, i32) = (-32, 96);
/// Margins in -8..0: every collapse is of negatives.
pub(super) const NEGATIVE: (i32, i32) = (-32, 0);

impl Random {
    pub(super) fn new(seed: u64, margins: (i32, i32)) -> Self {
        Self {
            state: seed,
            margins,
        }
    }

    fn next(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    pub(super) fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % bound as u64).expect("small")
    }

    /// A multiple of a quarter in `low..high` quarters.
    fn quarter(&mut self, low: i32, high: i32) -> f64 {
        let span = usize::try_from(high - low).expect("low under high");
        let drawn = i32::try_from(self.below(span)).expect("under the span");
        f64::from(low + drawn) / 4.0
    }

    fn margin(&mut self) -> f64 {
        self.quarter(self.margins.0, self.margins.1)
    }

    pub(super) fn child(&mut self) -> Child {
        let through = if self.below(4) == 0 {
            [self.margin(), self.margin()]
        } else {
            [0.0; 2]
        };
        Child {
            margins: [self.margin(), self.margin()],
            through,
            size: self.quarter(0, 96),
            overflow: [self.quarter(0, 96), self.quarter(0, 96)],
        }
    }
}

fn bounds(child: Child) -> Bounds {
    Bounds::span(child.low(), child.reach())
}

pub(super) fn block(child: Child) -> BlockSummary {
    let top = Margins::of(child.margins[0]).collapse(Margins::of(child.through[0]));
    let bottom = Margins::of(child.margins[1]).collapse(Margins::of(child.through[1]));
    BlockSummary::leaf(BlockChild {
        top,
        bottom,
        height: child.size,
        collapses_through: false,
        bounds: bounds(child),
    })
    .expect("leaf")
}

pub(super) fn flex(child: Child) -> FlexSummary {
    let (margins, size, bounds) = (child.margins, child.size, bounds(child));
    FlexSummary::leaf(FlexItem {
        margins,
        size,
        basis: size,
        content: size,
        bounds,
    })
}

/// Taffy's running pass: each child's start, with the sets between
/// neighbours collapsed set to set, and the first child's resolved.
pub(super) fn block_starts(children: &[Child]) -> Vec<f64> {
    let (mut y, mut bottom): (f64, Option<Set>) = (0.0, None);
    let mut starts = Vec::new();
    for child in children {
        let top = child.top_set();
        let between = bottom.map_or(top, |b| set(&[b.0, b.1, top.0, top.1]));
        y += between.0 + between.1;
        starts.push(y);
        y += child.size;
        bottom = Some(child.bottom_set());
    }
    starts
}

/// Each flex item's start: the sizes, margins and gaps before it.
pub(super) fn flex_starts(children: &[Child], gap: f64) -> Vec<f64> {
    let mut y = 0.0;
    let mut starts = Vec::new();
    for (index, child) in children.iter().enumerate() {
        y += if index > 0 { gap } else { 0.0 } + child.margins[0];
        starts.push(y);
        y += child.size + child.margins[1];
    }
    starts
}
