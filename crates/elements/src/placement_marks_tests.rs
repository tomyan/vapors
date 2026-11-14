use crate::layout::{Lanes, LayoutBox, Written};
use crate::placement::{LaidOut, Placement};

/// Slot 0 a container whose children are slots 1, 2 and 3, and no lane.
fn container_of_three() -> Vec<LayoutBox> {
    let mut boxes: Vec<LayoutBox> = (0..4).map(|_| LayoutBox::vacant()).collect();
    boxes[0].children = vec![1, 2, 3];
    boxes
}

/// The children `started_unframed` places for `started`.
fn placed_for(started: Vec<(usize, usize)>) -> Vec<usize> {
    let boxes = container_of_three();
    let lanes = Lanes::default();
    let scrolls = |_: usize| false;
    let laid = LaidOut::new(&boxes, &scrolls, &lanes, 1.0);
    let written = Written {
        started,
        ..Written::default()
    };
    Placement::default().started_unframed(&written, &laid)
}

#[test]
fn a_started_lane_dropped_since_places_its_children_from_the_first() {
    // When: a started entry whose container has no lane now
    let placed = placed_for(vec![(0, 2)]);

    // Then: its children from the one named on
    assert_eq!(placed, vec![2, 3]);
}

#[test]
#[should_panic(expected = "which is not one of its children")]
fn a_started_entry_naming_no_child_of_its_container_is_refused() {
    placed_for(vec![(0, 9)]);
}
