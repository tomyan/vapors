//! A flex column is laid out by its lane (`layout/lane_flex*.rs`,
//! `docs/plans/splice-layout.md`, section 10): an insert, a remove or a
//! resize in a column of rows asks the rows that changed and no other,
//! so the layout calls do not grow with the rows. Each case also
//! matches a full update.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::oracle::assert_matches_full;
use support::{document, element, text, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, LaneDecline, UpdateCounts, Viewport};

const VIEW: fn() -> Viewport = || viewport(640.0, 400.0);

/// The column, padded, with a gap; its rows 24px blocks with margins,
/// one a taller class.
const CSS: &str = "body { margin: 0; padding: 5px 0; display: flex; flex-direction: column; row-gap: 2px; } \
                   .row { height: 24px; margin: 3px 0 1px; flex-shrink: 0; } .tall { height: 48px; }";

/// A column of `rows` rows under body, laid out once.
fn column(rows: usize, css: &str) -> (Rc<RefCell<Document>>, Vec<NodeId>) {
    let mut doc = document();
    let body = doc.body();
    let nodes = (0..rows)
        .map(|index| {
            let row = element(&mut doc, body, "div", "row");
            text(&mut doc, row, &format!("Row {index}"));
            row
        })
        .collect();
    doc.add_stylesheet("column.css", css).expect("parses");
    doc.update(VIEW()).expect("updates");
    (Rc::new(RefCell::new(doc)), nodes)
}

/// The counts of the update after `edit` on a column of `rows`, which
/// matches a full update.
fn after(rows: usize, edit: impl Fn(&mut Document, &[NodeId])) -> UpdateCounts {
    let (doc, nodes) = column(rows, CSS);
    edit(&mut doc.borrow_mut(), &nodes);
    let counts = {
        let mut doc = doc.borrow_mut();
        doc.update(VIEW()).expect("updates");
        doc.update_counts()
    };
    assert_matches_full(&doc, VIEW(), "after the edit");
    counts
}

/// A new row put in before `rows[at]`.
fn insert(doc: &mut Document, rows: &[NodeId], at: usize) {
    let body = doc.body();
    let row = element(doc, body, "div", "row");
    text(doc, row, "new");
    doc.insert_before(body, row, Some(rows[at]));
}

#[test]
fn an_insert_asks_the_new_row_and_a_constant_whatever_the_rows() {
    // When
    let small = after(50, |doc, rows| insert(doc, rows, 25));
    let large = after(500, |doc, rows| insert(doc, rows, 250));

    // Then: the column's lane laid out the new row only
    assert_eq!(
        small.layout_calls, large.layout_calls,
        "layout calls grow with the rows"
    );
    assert!(small.lanes >= 1, "{small:?}");
    assert_eq!(small.children_edited, 1, "{small:?}");
}

#[test]
fn a_remove_asks_a_constant_whatever_the_rows() {
    let small = after(50, |doc, rows| doc.remove(rows[25]));
    let large = after(500, |doc, rows| doc.remove(rows[250]));
    assert_eq!(
        small.layout_calls, large.layout_calls,
        "layout calls grow with the rows"
    );
    assert!(small.lanes >= 1, "{small:?}");
}

#[test]
fn a_resized_row_asks_itself_and_a_constant_whatever_the_rows() {
    let resize = |at: usize| {
        move |doc: &mut Document, rows: &[NodeId]| doc.set_attribute(rows[at], "class", "row tall")
    };
    let small = after(50, resize(25));
    let large = after(500, resize(250));
    assert_eq!(
        small.layout_calls, large.layout_calls,
        "layout calls grow with the rows"
    );
    assert!(small.lanes >= 1, "{small:?}");
}

#[test]
fn a_column_whose_rows_would_shrink_is_declined_and_laid_out_by_taffy() {
    // Given: a column shorter than its rows, whose rows may shrink to
    // their text
    let css = "body { margin: 0; display: flex; flex-direction: column; height: 100px; } \
               .row { height: 24px; }";

    // When
    let (doc, _) = column(10, css);
    let counts = doc.borrow_mut().update_counts();

    // Then
    assert_eq!(
        counts.lane_decline,
        Some(LaneDecline::Shrinks),
        "{counts:?}"
    );
    assert_matches_full(&doc, VIEW(), "a shrinking column");
}

#[test]
fn a_column_inside_a_definite_height_column_asks_a_constant_whatever_the_rows() {
    // Given: the rows in a column that is an item of body, a column of
    // definite height, which asks it for its height and then lays it
    // out with that height known
    let calls = |rows: usize| {
        let mut doc = document();
        let body = doc.body();
        let list = element(&mut doc, body, "div", "list");
        let nodes: Vec<NodeId> = (0..rows)
            .map(|index| {
                let row = element(&mut doc, list, "div", "row");
                text(&mut doc, row, &format!("Row {index}"));
                row
            })
            .collect();
        let css = "body { margin: 0; display: flex; flex-direction: column; height: 400px; } \
                   .list { display: flex; flex-direction: column; flex-shrink: 0; } \
                   .row { height: 24px; flex-shrink: 0; }";
        doc.add_stylesheet("nested.css", css).expect("parses");
        doc.update(VIEW()).expect("updates");
        let doc = Rc::new(RefCell::new(doc));

        // When: a row inserted in the middle
        let row = element(&mut doc.borrow_mut(), list, "div", "row");
        doc.borrow_mut()
            .insert_before(list, row, Some(nodes[rows / 2]));
        doc.borrow_mut().update(VIEW()).expect("updates");
        let counts = doc.borrow_mut().update_counts();
        assert_matches_full(&doc, VIEW(), "nested insert");
        counts.layout_calls
    };

    // Then
    assert_eq!(calls(50), calls(500), "layout calls grow with the rows");
}
