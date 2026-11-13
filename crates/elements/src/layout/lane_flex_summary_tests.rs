//! Flex items sum with one gap between neighbours and no collapsing.

use super::{FlexItem, FlexSummary};
use crate::layout::lane_bounds::Bounds;
use crate::layout::lane_tree::Summary;

fn item(margins: [f64; 2], size: f64) -> FlexSummary {
    FlexSummary::leaf(FlexItem {
        margins,
        size,
        basis: size + 1.0,
        content: size + 2.0,
        bounds: Bounds::EMPTY,
    })
}

#[test]
fn margins_add_and_a_gap_sits_between_neighbours() {
    // Given
    let first = item([3.0, 8.0], 10.0);
    let second = item([2.0, 1.0], 20.0);

    // When
    let start = FlexSummary::start_of(&first, &second, 4.0);
    let run = first.combine(&second, 4.0);

    // Then: 3 + 10 + 8, a gap of 4, then 2
    assert_eq!(start, 27.0);
    assert_eq!(run.outer(4.0), 21.0 + 4.0 + 23.0);
}

#[test]
fn the_contributions_sum_with_their_gaps() {
    let run = item([0.0, 0.0], 10.0).combine(&item([0.0, 0.0], 20.0), 4.0);
    assert_eq!(run.contributions(4.0), [11.0 + 21.0 + 4.0, 12.0 + 22.0 + 4.0]);
}

#[test]
fn the_first_item_starts_at_its_start_margin() {
    let start = FlexSummary::start_of(&FlexSummary::default(), &item([6.0, 0.0], 10.0), 4.0);
    assert_eq!(start, 6.0);
}
