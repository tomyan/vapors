//! The walk from a frame to the page refuses frames it cannot follow:
//! a chain that never reaches the page, and a lane frame with no parent.

use super::frame_links;
use crate::clip_chain_frames::tests::nested_three;

/// The slots of the three nested scroll containers, outermost first.
fn scrollers(painted: &crate::painted::Painted) -> Vec<usize> {
    painted
        .frame_chains
        .iter()
        .map(|&(_, slot, _)| slot)
        .collect()
}

#[test]
#[should_panic(expected = "never reaches the page")]
fn a_frame_chain_that_comes_back_to_itself_is_refused() {
    // Given: the outermost scroller's frame set to the innermost, inside
    // it, so the walk outward from the innermost loops
    let (painted, mut order) = nested_three();
    let slots = scrollers(&painted);
    order.clips.set_frame(slots[0], slots[2]);

    // When
    frame_links(&order, &painted.frames, slots[2]);
}

#[test]
#[should_panic(expected = "has no parent box")]
fn a_lane_frame_with_no_parent_is_refused() {
    // Given: the root box, which has no parent, made a frame of its own
    // as a lane row is
    let (painted, mut order) = nested_three();
    order.clips.set_frame(0, 0);

    // When
    frame_links(&order, &painted.frames, 0);
}
