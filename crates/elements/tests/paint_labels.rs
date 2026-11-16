//! Paint-order labels through the splice (`docs/plans/paint-order-labels.md`):
//! rows put in at two places in turn take labels between their
//! neighbours', the splice relabelling a range when a gap runs out; rows
//! put in one after another at one place lean on the last and relabel
//! nothing, nor do rows appended, which step by the spacing. The
//! debug build holds the labels increasing along paint order after every
//! update (`kept_order_check.rs`), and each case matches a full update.

mod support;

use support::narrow_list::{List, VIEW, list, row};
use support::oracle::assert_matches_full;
use vapors_dom::Dom;

/// Puts a new row in `list` before the row `before` gives, `times`
/// times, updating each time; the labels the splices relabelled, summed.
fn put_in(
    list: &List,
    times: usize,
    before: impl Fn(&List) -> Option<vapors_dom::NodeId>,
) -> usize {
    let mut relabelled = 0;
    for _ in 0..times {
        let mut doc = list.doc.borrow_mut();
        let node = row(&mut doc, "new");
        doc.insert_before(list.parent, node, before(list));
        doc.update(VIEW()).expect("updates");
        let counts = doc.update_counts();
        assert_eq!(counts.order_splices, 1, "{counts:?}");
        relabelled += counts.paint_relabelled;
    }
    relabelled
}

#[test]
fn rows_put_in_at_two_places_in_turn_relabel_a_range_and_keep_the_order() {
    // Given
    let list = list(20, support::narrow_list::ROWS);

    // When: 150 rows put in before the sixth and the sixteenth in turn,
    // so no insert touches the one before and each halves a gap
    let turn = std::cell::Cell::new(0);
    let relabelled = put_in(&list, 150, |list| {
        turn.set(turn.get() + 1);
        Some(list.rows[if turn.get() % 2 == 0 { 5 } else { 15 }])
    });

    // Then: a gap ran out and a range was relabelled, the order kept
    assert!(
        relabelled > 0,
        "no gap ran out in 150 inserts at two places"
    );
    assert_matches_full(&list.doc, VIEW(), "after the inserts");
}

#[test]
fn rows_put_in_one_after_another_at_one_place_relabel_nothing() {
    // Given
    let list = list(20, support::narrow_list::ROWS);

    // When: 150 rows put in before the eleventh, each after the last
    let relabelled = put_in(&list, 150, |list| Some(list.rows[10]));

    // Then: each leaned on the last, and no gap ran out
    assert_eq!(relabelled, 0, "inserts after the last relabelled");
    assert_matches_full(&list.doc, VIEW(), "after the inserts");
}

#[test]
fn rows_appended_step_by_the_spacing_and_relabel_nothing() {
    // Given
    let list = list(20, support::narrow_list::ROWS);

    // When: 150 rows appended
    let relabelled = put_in(&list, 150, |_| None);

    // Then
    assert_eq!(relabelled, 0, "an append relabelled");
    assert_matches_full(&list.doc, VIEW(), "after the appends");
}
