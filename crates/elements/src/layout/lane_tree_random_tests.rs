//! Random inserts, removes and resizes fold to what a sequential pass
//! over the children gives, for block margins (Taffy 0.15.1's running
//! collapse) and for flex items (sums and gaps). Sizes are multiples of
//! a quarter, so every sum is exact.

use super::{Handle, LaneTree, Summary};
use crate::layout::lane_block_summary::{BlockChild, BlockSummary};
use crate::layout::lane_bounds::Bounds;
use crate::layout::lane_flex_summary::{FlexItem, FlexSummary};

/// A child as the sequential pass reads it.
#[derive(Clone, Copy, Debug)]
struct Child {
    margins: [f64; 2],
    size: f64,
    overflow: f64,
}

/// xorshift64, seeded: the same edits on every run.
struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % bound as u64).expect("small")
    }

    /// A multiple of a quarter in -8..24, or 0..24 when `positive`.
    fn quarter(&mut self, positive: bool) -> f64 {
        let low: u32 = if positive { 0 } else { 32 };
        let drawn = u32::try_from(self.below(96 + low as usize)).expect("under 128");
        (f64::from(drawn) - f64::from(low)) / 4.0
    }

    fn child(&mut self) -> Child {
        Child {
            margins: [self.quarter(false), self.quarter(false)],
            size: self.quarter(true),
            overflow: self.quarter(true),
        }
    }
}

fn bounds(child: Child) -> Bounds {
    Bounds {
        reach: child.size + child.overflow,
        ..Bounds::EMPTY
    }
}

fn block(child: Child) -> BlockSummary {
    let collapses_through = false;
    let (margins, height, bounds) = (child.margins, child.size, bounds(child));
    BlockSummary::leaf(BlockChild {
        margins,
        height,
        collapses_through,
        bounds,
    })
    .expect("leaf")
}

fn flex(child: Child) -> FlexSummary {
    let (margins, size, bounds) = (child.margins, child.size, bounds(child));
    FlexSummary::leaf(FlexItem {
        margins,
        size,
        basis: size,
        content: size,
        bounds,
    })
}

/// Taffy's running pass: each child's start, with margins collapsed.
fn block_starts(children: &[Child]) -> Vec<f64> {
    let collapse = |a: f64, b: f64| a.max(0.0).max(b.max(0.0)) + a.min(0.0).min(b.min(0.0));
    let (mut y, mut bottom) = (0.0, None);
    let mut starts = Vec::new();
    for child in children {
        y += bottom.map_or(child.margins[0], |b| collapse(b, child.margins[0]));
        starts.push(y);
        y += child.size;
        bottom = Some(child.margins[1]);
    }
    starts
}

/// Each flex item's start: the sizes, margins and gaps before it.
fn flex_starts(children: &[Child], gap: f64) -> Vec<f64> {
    let mut y = 0.0;
    let mut starts = Vec::new();
    for (index, child) in children.iter().enumerate() {
        y += if index > 0 { gap } else { 0.0 } + child.margins[0];
        starts.push(y);
        y += child.size + child.margins[1];
    }
    starts
}

/// Runs 2000 random edits on a tree of `leaf`, checking every start
/// and an interval's children against `starts` after each.
fn check<S: Summary>(
    seed: u64,
    gap: f64,
    leaf: fn(Child) -> S,
    starts: impl Fn(&[Child]) -> Vec<f64>,
) {
    let mut random = Random(seed);
    let mut tree = LaneTree::new(gap);
    let mut children: Vec<(Handle, Child)> = Vec::new();
    for step in 0..2000 {
        edit(&mut random, &mut tree, &mut children, leaf);
        let plain: Vec<Child> = children.iter().map(|&(_, c)| c).collect();
        let expected = starts(&plain);
        for (index, &(handle, _)) in children.iter().enumerate() {
            assert_eq!(
                tree.start(handle),
                expected[index],
                "seed {seed}, step {step}, child {index}"
            );
        }
        let order: Vec<Handle> = children.iter().map(|&(h, _)| h).collect();
        assert_eq!(tree.handles(), order, "seed {seed}, step {step}");
        check_between(&tree, &children, &expected, (seed, step));
    }
}

fn edit<S: Summary>(
    random: &mut Random,
    tree: &mut LaneTree<S>,
    children: &mut Vec<(Handle, Child)>,
    leaf: fn(Child) -> S,
) {
    let at = random.below(children.len() + 1);
    match random.below(10) {
        0..=4 => {
            let child = random.child();
            let previous = at.checked_sub(1).map(|i| children[i].0);
            children.insert(at, (tree.insert_after(previous, leaf(child)), child));
        }
        5..=7 if !children.is_empty() => tree.remove(children.remove(at.min(children.len() - 1)).0),
        _ if !children.is_empty() => {
            let at = at.min(children.len() - 1);
            let child = random.child();
            tree.set(children[at].0, leaf(child));
            children[at].1 = child;
        }
        _ => {}
    }
}

/// The children `between` gives for an interval and for the point at
/// each of a few children's starts, where a child that starts above its
/// run's first child (negative margins) is missed if the run's low
/// bound is not kept: every one whose box or overflow meets it, in
/// order.
fn check_between<S: Summary>(
    tree: &LaneTree<S>,
    children: &[(Handle, Child)],
    starts: &[f64],
    (seed, step): (u64, usize),
) {
    let from = f64::from(u32::try_from(step % 97).expect("under 97")) * 3.0;
    let mut probes = vec![(from, from + 40.0)];
    probes.extend(starts.iter().step_by(7).map(|&start| (start, start)));
    for (from, to) in probes {
        let meets = |(index, &(handle, child)): (usize, &(Handle, Child))| {
            let reach = child.size + child.overflow;
            (starts[index] <= to && starts[index] + reach >= from).then_some(handle)
        };
        let expected: Vec<Handle> = children.iter().enumerate().filter_map(meets).collect();
        assert_eq!(
            tree.between(from, to),
            expected,
            "seed {seed}, step {step}, {from}..{to}"
        );
    }
}

#[test]
fn block_margins_fold_as_a_running_collapse_over_random_edits() {
    for seed in [1, 7, 42] {
        check(seed, 0.0, block, block_starts);
    }
}

#[test]
fn flex_items_fold_as_sums_and_gaps_over_random_edits() {
    for seed in [3, 11, 99] {
        check(seed, 2.5, flex, |children| flex_starts(children, 2.5));
    }
}
