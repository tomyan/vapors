//! Many rows removed, inserted and moved in one update restyle to the
//! same styles as a page built afresh in the final order, under each
//! kind of sibling reach (entry 117, action 48): the coalesced sibling
//! marks keep the restyle's result. Debug builds also run
//! `restyle_check` on each of these updates.

use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;

use crate::support;

use support::basics::{document, element, viewport};

const ROWS: usize = 24;

/// Each sheet reaches siblings another way: later, earlier, a subtree,
/// a bounded count, or several at once.
const SHEETS: [&str; 7] = [
    ".row:nth-child(odd) { color: rgb(200 0 0); }",
    ".row:nth-child(3n + 1) .cell { color: rgb(0 200 0); }",
    ".row:nth-last-child(odd) { color: rgb(0 0 200); }",
    ".row.mark ~ .row { color: rgb(9 9 9); }",
    ".row + .row + .row { margin-top: 1px; }",
    ".row:nth-of-type(even) .cell { color: rgb(7 7 7); } .row:last-child { color: rgb(1 1 1); }",
    ".row:nth-child(odd) { color: rgb(200 0 0); } .row:nth-last-child(3n) .cell { color: rgb(3 3 3); } .row.mark + .row { color: rgb(5 5 5); }",
];

/// A row of class `row` (`row mark` when `marked`) with one cell.
fn row(doc: &mut Document, parent: NodeId, marked: bool) -> NodeId {
    let class = if marked { "row mark" } else { "row" };
    let row = element(doc, parent, "div", class);
    element(doc, row, "div", "cell");
    row
}

/// The column with `marks.len()` rows, each marked as `marks` says, under `css`.
fn page(css: &str, marks: &[bool]) -> (Document, NodeId, Vec<NodeId>) {
    let mut doc = document();
    let body = doc.body();
    let column = element(&mut doc, body, "div", "column");
    let rows = marks.iter().map(|&m| row(&mut doc, column, m)).collect();
    doc.add_stylesheet("app.css", css).expect("parses");
    doc.update(viewport(400.0, 2000.0)).expect("updates");
    (doc, column, rows)
}

/// Removes every third row, inserts three new rows (one marked), moves
/// the last original row, which the removals' walks reached, to the
/// front, then appends a fourth new row; returns the marks in the
/// final order.
fn edit(doc: &mut Document, column: NodeId, rows: &[NodeId], marks: &[bool]) -> Vec<bool> {
    let mut order: Vec<(NodeId, bool)> = rows.iter().copied().zip(marks.iter().copied()).collect();
    for i in (0..ROWS).step_by(3) {
        doc.remove(rows[i]);
    }
    order.retain(|(node, _)| !rows.iter().step_by(3).any(|r| r == node));
    for (at, marked) in [(1, false), (5, true), (9, false)] {
        insert_new(doc, column, &mut order, at, marked);
    }
    let last = order.pop().expect("rows remain");
    assert_eq!(
        last.0,
        rows[ROWS - 1],
        "the row moved is the last original row"
    );
    doc.insert_before(column, last.0, Some(order[0].0));
    order.insert(0, last);
    // `usize::MAX`: appended, past the last row
    insert_new(doc, column, &mut order, usize::MAX, false);
    order.into_iter().map(|o| o.1).collect()
}

/// A new row with one cell inserted at `at` of `order`, or appended
/// past its end, in the document and in `order`.
fn insert_new(
    doc: &mut Document,
    column: NodeId,
    order: &mut Vec<(NodeId, bool)>,
    at: usize,
    marked: bool,
) {
    let new = doc.create_element("div");
    doc.set_attribute(new, "class", if marked { "row mark" } else { "row" });
    let cell = doc.create_element("div");
    doc.set_attribute(cell, "class", "cell");
    doc.append(new, cell);
    doc.insert_before(column, new, order.get(at).map(|o| o.0));
    order.insert(at.min(order.len()), (new, marked));
}

/// Each row's and its cell's computed style, in the column's order.
fn styles(doc: &Document, column: NodeId) -> Vec<String> {
    let mut out = Vec::new();
    for row in doc.children(column) {
        for node in std::iter::once(row).chain(doc.children(row)) {
            out.push(format!("{:?}", doc.computed_style(node).expect("styled")));
        }
    }
    out
}

#[test]
fn many_rows_changed_in_one_update_restyle_as_a_fresh_page_under_every_sibling_reach() {
    for css in SHEETS {
        // Given
        let marks: Vec<bool> = (0..ROWS).map(|i| i == 4).collect();
        let (mut doc, column, rows) = page(css, &marks);

        // When
        let final_marks = edit(&mut doc, column, &rows, &marks);
        doc.update(viewport(400.0, 2000.0)).expect("updates");

        // Then
        let (fresh, fresh_column, _) = page(css, &final_marks);
        assert_eq!(
            styles(&doc, column),
            styles(&fresh, fresh_column),
            "under {css}"
        );
    }
}

#[test]
fn a_late_row_moved_early_then_a_change_before_it_restyles_as_a_fresh_page() {
    // Given
    let css = ".row:nth-child(odd) { color: rgb(200 0 0); }";
    let marks = vec![false; ROWS];
    let (mut doc, column, rows) = page(css, &marks);

    // When: a late removal walks the rows after it; one of them moves
    // early; then a removal before it walks up to it
    doc.remove(rows[20]);
    doc.insert_before(column, rows[22], Some(rows[3]));
    doc.remove(rows[1]);
    doc.update(viewport(400.0, 2000.0)).expect("updates");

    // Then
    let (fresh, fresh_column, _) = page(css, &vec![false; ROWS - 2]);
    assert_eq!(styles(&doc, column), styles(&fresh, fresh_column));
}
