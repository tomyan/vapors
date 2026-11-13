//! Block margins fold as Taffy 0.15.1 collapses them between siblings.

use super::{BlockChild, BlockSummary, LaneDecline};
use crate::layout::lane_bounds::Bounds;
use crate::layout::lane_tree::Summary;

fn child(margins: [f64; 2], height: f64) -> BlockSummary {
    BlockSummary::leaf(BlockChild {
        margins,
        height,
        collapses_through: false,
        bounds: Bounds::EMPTY,
    })
    .expect("does not collapse through")
}

/// Where the second of two children starts.
fn second_start(first: [f64; 2], second: [f64; 2]) -> f64 {
    BlockSummary::start_of(&child(first, 10.0), &child(second, 10.0), 0.0)
}

#[test]
fn neighbouring_margins_collapse_to_the_larger_the_sum_or_the_more_negative() {
    let cases = [
        ([0.0, 6.0], [4.0, 0.0], 16.0),  // both positive: the larger
        ([0.0, 6.0], [-4.0, 0.0], 12.0), // mixed: the sum
        ([0.0, -6.0], [-4.0, 0.0], 4.0), // both negative: the more negative
    ];
    for (first, second, start) in cases {
        assert_eq!(second_start(first, second), start, "{first:?} {second:?}");
    }
}

#[test]
fn the_first_child_starts_at_its_top_margin() {
    let start = BlockSummary::start_of(&BlockSummary::default(), &child([5.0, 0.0], 10.0), 0.0);
    assert_eq!(start, 5.0);
}

#[test]
fn a_run_keeps_its_outer_margins_and_collapses_the_inner() {
    // Given
    let run = child([3.0, 8.0], 10.0).combine(&child([2.0, 1.0], 20.0), 0.0);

    // Then: 3 + 10 + max(8, 2) + 20 + 1
    assert_eq!(run.outer(), 42.0);
}

#[test]
fn a_child_that_collapses_through_is_declined() {
    let leaf = BlockSummary::leaf(BlockChild {
        margins: [4.0, 4.0],
        height: 0.0,
        collapses_through: true,
        bounds: Bounds::EMPTY,
    });
    assert_eq!(leaf, Err(LaneDecline::CollapsesThrough));
}
