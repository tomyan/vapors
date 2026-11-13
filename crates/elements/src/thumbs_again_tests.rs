use crate::changed_slots::ChangedSlots;
use crate::frames::PAGE;
use crate::kept_box::BoxFrom;
use crate::layout::BoxRect;

use super::thumbs_again;

/// Slots 1, 3, 5 and 7 scroll.
fn scrolls(slot: usize) -> bool {
    slot % 2 == 1
}

/// Changes naming `settled` as settled by placement, nothing else.
fn settled(settled: &[usize]) -> ChangedSlots {
    let mut changed = ChangedSlots::all();
    changed.every = false;
    changed.settled = settled.to_vec();
    changed
}

/// `slots` as painted again, each made from an empty box on the page.
fn again(slots: &[usize]) -> Vec<(usize, BoxFrom)> {
    let from = BoxFrom {
        rect: BoxRect {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        },
        clip: None,
        padding: taffy::Rect::zero(),
        frame: PAGE,
    };
    slots.iter().map(|&slot| (slot, from)).collect()
}

#[test]
fn every_thumb_is_painted_when_nothing_was_kept_or_thumbs_were_toggled() {
    // Given
    let thumbs = [1, 3, 5, 7];

    // When
    let found = thumbs_again(&thumbs, (&[], &[]), &settled(&[]), true, &scrolls);

    // Then
    assert_eq!(found, thumbs);
}

#[test]
fn only_the_containers_painted_again_shifted_or_settled_are_painted() {
    // Given: 3 painted again, 5 shifted, 7 settled; 1 untouched
    let (again, shifted) = (again(&[3]), [5]);

    // When
    let found = thumbs_again(
        &[1, 3, 5, 7],
        (&again, &shifted),
        &settled(&[7]),
        false,
        &scrolls,
    );

    // Then
    assert_eq!(found, [3, 5, 7]);
}

#[test]
fn slots_that_do_not_scroll_are_left_out_and_each_container_is_named_once() {
    // Given: 2 and 4 do not scroll; 3 is painted again, shifted and settled
    let (again, shifted) = (again(&[2, 3]), [3, 4]);

    // When
    let found = thumbs_again(&[1, 3], (&again, &shifted), &settled(&[3]), false, &scrolls);

    // Then
    assert_eq!(found, [3]);
}
