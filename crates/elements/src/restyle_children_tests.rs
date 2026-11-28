//! Many rows inserted or removed in one update mark each sibling the
//! sheets reach at most once per direction (entry 117, action 48): the
//! marks are bounded by the parent's children and the changes, not by
//! their product.

use vapors_dom::{CompiledSheet, Dom, NodeId};
use vapors_text::FontSystem;

use crate::document::Document;
use crate::test_rows::viewport;

/// The rows under the body.
const ROWS: usize = 200;
/// Every `STEP`-th row is removed, or has a row inserted before it.
const STEP: usize = 4;
const CHANGES: usize = ROWS / STEP;

/// `ROWS` rows of class `r`, each with one child of class `c`, mounted
/// under `css` by one update.
fn mounted(css: &str) -> (Document, Vec<NodeId>) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_style(CompiledSheet::for_tests("rows", css));
    let body = doc.body();
    let rows = (0..ROWS)
        .map(|_| {
            let row = doc.create_element("div");
            doc.set_attribute(row, "class", "r");
            let cell = doc.create_element("div");
            doc.set_attribute(cell, "class", "c");
            doc.append(row, cell);
            doc.append(body, row);
            row
        })
        .collect();
    doc.update(viewport()).expect("mounts");
    (doc, rows)
}

fn remove_every_step(doc: &mut Document, rows: &[NodeId]) {
    for row in rows.iter().step_by(STEP) {
        doc.remove(*row);
    }
}

fn insert_every_step(doc: &mut Document, rows: &[NodeId]) {
    let body = doc.body();
    for before in rows.iter().step_by(STEP) {
        let row = doc.create_element("div");
        doc.set_attribute(row, "class", "r");
        doc.insert_before(body, row, Some(*before));
    }
}

/// The marks so far: single elements and subtree roots, each as pushed.
fn marks(doc: &Document) -> (usize, usize) {
    (doc.dirty.elements.len(), doc.dirty.roots.len())
}

#[test]
fn removing_rows_under_a_subject_nth_child_marks_each_later_row_once() {
    // Given
    let (mut doc, rows) = mounted(".r:nth-child(odd) { color: rgb(1 2 3); }");

    // When
    remove_every_step(&mut doc, &rows);

    // Then: at most every row once, where each removal re-marking the rows after it is ROWS * CHANGES / 2
    let (elements, roots) = marks(&doc);
    assert!(
        elements <= ROWS && roots == 0,
        "{elements} element and {roots} root marks for {CHANGES} removals of {ROWS} rows"
    );
}

#[test]
fn inserting_rows_under_a_subject_nth_child_marks_each_later_row_once() {
    // Given
    let (mut doc, rows) = mounted(".r:nth-child(odd) { color: rgb(1 2 3); }");

    // When
    insert_every_step(&mut doc, &rows);

    // Then: each inserted row is a root, twice, by its class set
    // (`mark_changed`) and by its insert, and has no element mark; each
    // original row from the first insert's place on is marked once
    let (elements, roots) = marks(&doc);
    assert!(
        elements <= ROWS && roots == 2 * CHANGES,
        "{elements} element and {roots} root marks for {CHANGES} inserts among {ROWS} rows"
    );
}

#[test]
fn removing_rows_under_an_ancestor_nth_child_marks_each_later_subtree_once() {
    // Given: the positional pseudo-class in the group before the subject's
    let (mut doc, rows) = mounted(".r:nth-child(odd) .c { color: rgb(1 2 3); }");

    // When
    remove_every_step(&mut doc, &rows);

    // Then
    let (elements, roots) = marks(&doc);
    assert!(
        elements == 0 && roots <= ROWS,
        "{elements} element and {roots} root marks for {CHANGES} removals of {ROWS} rows"
    );
}

#[test]
fn removing_rows_under_nth_last_child_marks_each_earlier_row_once() {
    // Given
    let (mut doc, rows) = mounted(".r:nth-last-child(odd) { color: rgb(1 2 3); }");

    // When
    remove_every_step(&mut doc, &rows);

    // Then
    let (elements, roots) = marks(&doc);
    assert!(
        elements <= ROWS && roots == 0,
        "{elements} element and {roots} root marks for {CHANGES} removals of {ROWS} rows"
    );
}
