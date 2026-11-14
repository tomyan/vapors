//! A flex column whose items read its height inputs, and a column whose
//! items' heights depend on how it is asked (`layout/lane_flex*.rs`;
//! the review of #343). Each page also matches a full update.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::oracle::assert_matches_full;
use support::{document, element, text, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, UpdateCounts, Viewport};

const VIEW: fn() -> Viewport = || viewport(640.0, 400.0);

/// `rows` rows of text in a `.list` under body, styled by `css`,
/// laid out once.
fn nested(css: &str, rows: usize) -> (Rc<RefCell<Document>>, NodeId, Vec<NodeId>) {
    let mut doc = document();
    let body = doc.body();
    let list = element(&mut doc, body, "div", "list");
    let nodes = (0..rows)
        .map(|index| {
            let row = element(&mut doc, list, "div", "row");
            text(&mut doc, row, &format!("Row {index}"));
            row
        })
        .collect();
    doc.add_stylesheet("nested.css", css).expect("parses");
    doc.update(VIEW()).expect("updates");
    (Rc::new(RefCell::new(doc)), list, nodes)
}

/// The counts of the update after a row is put in before `before`.
fn insert(doc: &Rc<RefCell<Document>>, list: NodeId, before: NodeId) -> UpdateCounts {
    let mut doc = doc.borrow_mut();
    let row = element(&mut doc, list, "div", "row");
    text(&mut doc, row, "new");
    doc.insert_before(list, row, Some(before));
    doc.update(VIEW()).expect("updates");
    doc.update_counts()
}

#[test]
fn rows_with_a_percentage_flex_basis_are_laid_out_again_when_the_column_height_is_known() {
    // Given: rows whose `flex-basis` is a percentage of their column's
    // height, the column an item of a definite-height column, so asked
    // first with its height unknown, then laid out with it known
    let css = "body { margin: 0; display: flex; flex-direction: column; height: 400px; } \
               .list { display: flex; flex-direction: column; flex-shrink: 0; } \
               .row { flex-basis: 50%; flex-shrink: 0; }";
    let (doc, list, rows) = nested(css, 5);

    // When: a row put in, so the others are not stale
    insert(&doc, list, rows[2]);

    // Then: each row's leaf was made again for the layout (the debug
    // layout check in the update would have panicked otherwise)
    assert_matches_full(&doc, VIEW(), "percentage flex-basis, insert");
}

/// The layout calls of an insert in the middle of `rows` rows.
fn nested_insert_calls(css: &str, rows: usize) -> usize {
    let (doc, list, nodes) = nested(css, rows);
    let counts = insert(&doc, list, nodes[rows / 2]);
    assert_matches_full(&doc, VIEW(), "nested insert");
    counts.layout_calls
}

/// Touche's shell one level deeper: a definite-height column that is an
/// item of a flex column, its rows reading its height. A known O(rows)
/// case (`docs/plans/splice-layout.md`, 10.6): the parent asks the
/// column for its content size, an ask that drops its styled height, so
/// its height inputs flip between that ask and the layout and every row
/// reading them is laid out again on each. Pinned at what it costs, so
/// a change in either direction is seen.
#[test]
fn a_definite_column_inside_a_flex_column_lays_out_its_reading_rows_on_every_ask() {
    let css = "body { margin: 0; display: flex; flex-direction: column; height: 400px; } \
               .list { display: flex; flex-direction: column; height: 300px; flex-shrink: 0; } \
               .row { min-height: 10%; flex-shrink: 0; }";
    assert_eq!(
        (nested_insert_calls(css, 50), nested_insert_calls(css, 500)),
        (KNOWN_AT_50, KNOWN_AT_500)
    );
}

/// The pinned counts of the case above, measured at this commit (the
/// review measured 373 and 3523 before each child was laid out once per
/// ask).
const KNOWN_AT_50: usize = 366;
const KNOWN_AT_500: usize = 3516;

#[test]
fn a_column_of_wrapping_columns_is_declined_as_space_dependent() {
    // Given: rows that are wrapping flex columns, whose height under
    // min-content space along differs from under max-content (the
    // review's page)
    let css = "body { margin: 0; } \
               .list { display: flex; flex-direction: column; } \
               .row { display: flex; flex-direction: column; flex-wrap: wrap; flex-shrink: 0; } \
               .cell { height: 20px; width: 50px; flex-shrink: 0; }";
    let mut doc = document();
    let body = doc.body();
    let list = element(&mut doc, body, "div", "list");
    for _ in 0..3 {
        let row = element(&mut doc, list, "div", "row");
        for _ in 0..3 {
            element(&mut doc, row, "div", "cell");
        }
    }
    doc.add_stylesheet("wrap.css", css).expect("parses");

    // When
    doc.update(VIEW()).expect("updates");
    let counts = doc.update_counts();

    // Then: body is the one lane; the list is declined for its rows'
    // heights (the counts name only the first decline, a row's
    // `FlexWrap`; the list is a lane, 2 in all, without the check)
    assert_eq!((counts.lanes, counts.lanes_declined), (1, 10), "{counts:?}");
    let doc = Rc::new(RefCell::new(doc));
    assert_matches_full(&doc, VIEW(), "wrapping rows");
}
