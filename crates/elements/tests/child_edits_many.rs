//! Many rows inserted under one list in one update put each in its
//! place with bounded work (entry 122, action 49): the keyed filter of
//! gui-bench's dashboard brings back thousands of rows at once, and
//! ordering the inserts compared each pending row with every other.
//! Each case also matches a full update.

use crate::support;

use support::narrow_list::{List, ROWS, VIEW, list, row};
use support::oracle::assert_matches_full;
use vapors_dom::{Dom, NodeId};

const COUNT: usize = 400;
/// Every `KEPT`-th row stays; the others are removed, then brought back.
const KEPT: usize = 8;

/// Removes every row but each `KEPT`-th, and updates.
fn filtered() -> (List, Vec<usize>) {
    let list = list(COUNT, ROWS);
    let removed: Vec<usize> = (0..COUNT).filter(|i| i % KEPT != 0).collect();
    for &i in &removed {
        list.doc.borrow_mut().remove(list.rows[i]);
    }
    list.update();
    (list, removed)
}

/// A fresh row for position `i`, put before the next present row: a
/// removed node is freed, so a keyed list brings a row back as a new one.
fn put_back(list: &List, i: usize, present: &[Option<NodeId>]) -> NodeId {
    let before = present[i + 1..].iter().flatten().next().copied();
    let mut doc = list.doc.borrow_mut();
    let node = row(&mut doc, &format!("row {i}"));
    doc.insert_before(list.parent, node, before);
    node
}

/// Brings `removed` back in `order`, updates, and returns the child
/// entries scanned; the list's children were edited, not rebuilt.
fn brought_back(list: &List, removed: &[usize], order: impl Iterator<Item = usize>) -> usize {
    let mut present: Vec<Option<NodeId>> = (0..COUNT)
        .map(|i| (!removed.contains(&i)).then_some(list.rows[i]))
        .collect();
    for i in order {
        present[i] = Some(put_back(list, i, &present));
    }
    edited_scanned(list)
}

/// The update's child entries scanned, after checking it edited the
/// list's children in place: a rebuild would scan none of them.
fn edited_scanned(list: &List) -> usize {
    let counts = list.update();
    assert_eq!(
        counts.children_edited, 1,
        "the list's children were rebuilt, not edited"
    );
    counts.children_scanned
}

#[test]
fn rows_brought_back_last_first_are_ordered_with_bounded_work() {
    // Given: 50 of 400 rows left, as a filter leaves them
    let (list, removed) = filtered();

    // When: the 350 come back from the last, each before the row after it (a keyed list's order)
    let scanned = brought_back(&list, &removed, removed.clone().into_iter().rev());

    // Then: work linear in the rows, where comparing each pending row with the rest is about 350² / 2
    assert!(
        scanned <= COUNT,
        "{scanned} child entries scanned bringing back {} of {COUNT} rows",
        removed.len()
    );
    assert_matches_full(&list.doc, VIEW(), "rows brought back last first");
}

#[test]
fn rows_brought_back_first_first_are_ordered_with_bounded_work() {
    // Given
    let (list, removed) = filtered();

    // When
    let scanned = brought_back(&list, &removed, removed.clone().into_iter());

    // Then
    assert!(
        scanned <= COUNT,
        "{scanned} child entries scanned bringing back {} of {COUNT} rows",
        removed.len()
    );
    assert_matches_full(&list.doc, VIEW(), "rows brought back first first");
}

#[test]
fn rows_brought_back_in_a_scattered_order_land_in_the_lists_order() {
    // Given
    let (list, removed) = filtered();

    // When: odd positions first, then the rest from the last
    let odd = removed.iter().copied().filter(|i| i % 2 == 1);
    let even = removed.iter().copied().filter(|i| i % 2 == 0).rev();
    brought_back(&list, &removed, odd.chain(even));

    // Then: the boxes are those of a full update
    assert_matches_full(&list.doc, VIEW(), "rows brought back in a scattered order");
}

#[test]
fn every_row_inserted_into_an_empty_list_is_ordered_with_bounded_work() {
    // Given: every row removed
    let list = list(COUNT, ROWS);
    for &row in &list.rows {
        list.doc.borrow_mut().remove(row);
    }
    list.update();

    // When: as many fresh rows back from the last, one run of inserts with no box before it
    for i in (0..COUNT).rev() {
        let mut doc = list.doc.borrow_mut();
        let first = doc.children(list.parent).first().copied();
        let node = row(&mut doc, &format!("row {i}"));
        doc.insert_before(list.parent, node, first);
    }
    let scanned = edited_scanned(&list);

    // Then
    assert!(
        scanned <= COUNT,
        "{scanned} child entries scanned inserting {COUNT} rows into an empty list"
    );
    assert_matches_full(&list.doc, VIEW(), "every row inserted into an empty list");
}
