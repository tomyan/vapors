//! Block margins fold as Taffy 0.15.1 collapses them between siblings.

use super::{BlockChild, BlockSummary, LaneDecline, Margins};
use crate::layout::lane_bounds::Bounds;
use crate::layout::lane_tree::Summary;

/// A child with `top` and `bottom` margin sets, each of one margin.
fn child(margins: [f64; 2], height: f64) -> BlockSummary {
    sets(Margins::of(margins[0]), Margins::of(margins[1]), height)
}

fn sets(top: Margins, bottom: Margins, height: f64) -> BlockSummary {
    BlockSummary::leaf(BlockChild {
        top,
        bottom,
        height,
        collapses_through: false,
        bounds: Bounds::span(0.0, height),
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
fn a_childs_set_collapses_as_a_set_not_as_its_resolved_margin() {
    // Given: `<div style="margin-top: -5px"><p style="margin-top: 20px">`
    // with nothing between: a top set of {20, -5}, after a bottom of 18
    let top = Margins::of(-5.0).collapse(Margins::of(20.0));
    let second = sets(top, Margins::default(), 10.0);

    // When
    let start = BlockSummary::start_of(&child([0.0, 18.0], 10.0), &second, 0.0);

    // Then: max(20, 18) + min(-5, 0) = 15, not max(15, 18) = 18
    assert_eq!(start, 25.0);
}

#[test]
fn a_run_exposes_its_first_top_and_last_bottom_sets() {
    let run = child([3.0, -8.0], 10.0).combine(&child([2.0, 1.0], 20.0), 0.0);
    assert_eq!(run.top(), Margins::of(3.0));
    assert_eq!(run.bottom(), Margins::of(1.0));
    assert_eq!(BlockSummary::default().top(), Margins::default());
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
        top: Margins::of(4.0),
        bottom: Margins::of(4.0),
        height: 0.0,
        collapses_through: true,
        bounds: Bounds::span(0.0, 0.0),
    });
    assert_eq!(leaf, Err(LaneDecline::CollapsesThrough));
}
