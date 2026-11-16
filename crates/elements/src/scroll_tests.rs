//! A container that only scrolled is clamped to the range it kept (4g).

use vapors_dom::NodeId;

use super::Offsets;

#[test]
#[should_panic(expected = "is marked as only scrolled but kept no offset")]
fn settling_a_container_that_kept_no_offset_as_only_scrolled_is_refused() {
    // Given: no container settled
    let mut offsets = Offsets::default();

    // When
    offsets.settle_scrolled(NodeId(4), 1.0);
}
