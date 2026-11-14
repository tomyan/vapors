//! A lane's record of its children between layouts.

use super::Lane;
use crate::layout::lane_block_summary::{BlockChild, BlockSummary, Margins};
use crate::layout::lane_bounds::Bounds;

/// A lane holding children in `slots`, in order, each 10 high, with
/// nothing moved since.
fn lane_of(slots: &[usize]) -> Lane {
    let mut lane = Lane::default();
    let mut previous = None;
    for &slot in slots {
        previous = Some(lane.put(slot, previous, row()));
    }
    lane.moved.clear();
    lane
}

fn row() -> BlockSummary {
    BlockSummary::leaf(BlockChild {
        top: Margins::of(0.0),
        bottom: Margins::of(0.0),
        height: 10.0,
        collapses_through: false,
        bounds: Bounds::span(0.0, 10.0),
    })
    .expect("does not collapse through")
}

#[test]
fn neighbours_removed_first_to_last_leave_only_the_next_child_moved() {
    // Given
    let mut lane = lane_of(&[25, 26, 27]);

    // When: 25 goes, then 26, the child that had moved into its place
    lane.child_removed(25, Some(26));
    lane.child_removed(26, Some(27));

    // Then: 27 moved, now first; the removed 26 is not asked for
    assert_eq!(lane.moved, [27]);
    assert_eq!(lane.first_moved(), Some(lane.handle(27)));
}

#[test]
fn a_child_inserted_and_removed_before_a_layout_is_not_put() {
    // Given
    let mut lane = lane_of(&[1]);
    lane.child_inserted(2, Some(1));

    // When
    lane.child_removed(2, None);

    // Then
    assert_eq!(lane.inserted, []);
}
