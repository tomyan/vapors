//! `spliced_steps`: edits at their places in the order before any edit.

use super::spliced_steps;
use crate::order_splice::Edit;
use crate::paint_order::Step;

fn boxes(slots: &[usize]) -> Vec<Step> {
    slots.iter().map(|&slot| Step::Box(slot)).collect()
}

#[test]
fn an_insertion_and_a_removal_at_the_same_place_keep_the_inserted_steps() {
    // Given: box 2 removed where 9 goes in, after 1
    let steps = boxes(&[0, 1, 2, 3]);
    let edits = vec![
        Edit {
            at: 2,
            removed: 1,
            inserted: Vec::new(),
        },
        Edit {
            at: 2,
            removed: 0,
            inserted: boxes(&[9]),
        },
    ];

    // When
    let out = spliced_steps(&steps, edits);

    // Then
    assert_eq!(out, boxes(&[0, 1, 9, 3]));
}

#[test]
fn edits_are_made_at_their_places_before_any_edit() {
    // Given: 8 before box 1, box 2 out, 9 before box 3
    let steps = boxes(&[0, 1, 2, 3]);
    let edits = vec![
        Edit {
            at: 3,
            removed: 0,
            inserted: boxes(&[9]),
        },
        Edit {
            at: 1,
            removed: 0,
            inserted: boxes(&[8]),
        },
        Edit {
            at: 2,
            removed: 1,
            inserted: Vec::new(),
        },
    ];

    // When
    let out = spliced_steps(&steps, edits);

    // Then
    assert_eq!(out, boxes(&[0, 8, 1, 9, 3]));
}

#[test]
#[should_panic(expected = "order splice edits overlap at step 2")]
fn overlapping_removals_panic_naming_the_step() {
    // Given: two removals over box 2
    let steps = boxes(&[0, 1, 2, 3]);
    let edits = vec![
        Edit {
            at: 1,
            removed: 2,
            inserted: Vec::new(),
        },
        Edit {
            at: 2,
            removed: 1,
            inserted: Vec::new(),
        },
    ];

    // When / Then
    spliced_steps(&steps, edits);
}
