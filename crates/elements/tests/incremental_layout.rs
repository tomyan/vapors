//! Layout is kept between updates (F2c S7): an unchanged box takes its
//! last cache and layout, so an update lays out only what changed and
//! its ancestors; and the result is what a fresh document lays out.

mod support;

use support::{document, element, text, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{BoxRect, Document};

const CSS: &str = "body { margin: 0; } .row { display: flex; gap: 4px; } .cell { padding: 2px; } .wide { width: 90px; }";

/// body > column > 6 rows of 4 cells with text; the cells and their
/// text nodes, in order.
fn page() -> (Document, Vec<NodeId>, Vec<NodeId>) {
    let mut doc = document();
    let body = doc.body();
    let column = element(&mut doc, body, "div", "column");
    let (mut cells, mut texts) = (Vec::new(), Vec::new());
    for r in 0..6 {
        let row = element(&mut doc, column, "div", "row");
        for c in 0..4 {
            let cell = element(&mut doc, row, "div", "cell");
            texts.push(text(&mut doc, cell, &format!("{r}.{c}")));
            cells.push(cell);
        }
    }
    doc.add_stylesheet("app.css", CSS).expect("parses");
    (doc, cells, texts)
}

/// The changes the correctness test makes, in order.
fn change(doc: &mut Document, cells: &[NodeId], texts: &[NodeId], step: usize) {
    match step {
        0 => doc.set_text(texts[5], "a much longer cell text"),
        1 => doc.set_attribute(cells[9], "class", "cell wide"),
        2 => {
            let extra = element(doc, doc.parent(cells[13]).expect("a row"), "div", "cell");
            text(doc, extra, "new");
        }
        3 => doc.remove(cells[2]),
        _ => doc.set_text(texts[20], "x"),
    }
}

fn boxes(doc: &mut Document) -> Vec<(NodeId, BoxRect)> {
    let nodes = doc.elements();
    let painted = doc.update(viewport(300.0, 400.0)).expect("updates");
    nodes
        .into_iter()
        .map(|node| (node, painted.border_box(node).expect("a box")))
        .collect()
}

#[test]
fn an_update_with_nothing_changed_lays_out_no_box() {
    // Given
    let (mut doc, _, _) = page();
    doc.update(viewport(300.0, 400.0)).expect("updates");

    // When
    doc.update(viewport(300.0, 400.0)).expect("updates");

    // Then
    assert_eq!(doc.boxes_laid_out(), 0);
}

#[test]
fn a_changed_text_lays_out_its_box_and_its_ancestors_only() {
    // Given
    let (mut doc, _, texts) = page();
    doc.update(viewport(300.0, 400.0)).expect("updates");
    let first = doc.boxes_laid_out();

    // When
    doc.set_text(texts[5], "6");
    doc.update(viewport(300.0, 400.0)).expect("updates");

    // Then: only the changed text's chain
    assert_eq!(
        doc.boxes_laid_out(),
        TEXT_CHANGE_LAYOUTS,
        "the first update laid out {first}"
    );
}

/// Measured (2026-10-23): 12 layouts for the six boxes on the changed
/// text's chain (Taffy lays some out more than once: a measure and a
/// layout pass), where the first update made 201.
const TEXT_CHANGE_LAYOUTS: usize = 12;

#[test]
fn after_each_change_the_boxes_are_a_fresh_documents() {
    // Given
    let (mut kept, cells, texts) = page();
    kept.update(viewport(300.0, 400.0)).expect("updates");

    for step in 0..5 {
        // When: the change, to the kept document and to a fresh one
        change(&mut kept, &cells, &texts, step);
        let (mut fresh, fresh_cells, fresh_texts) = page();
        for earlier in 0..=step {
            change(&mut fresh, &fresh_cells, &fresh_texts, earlier);
        }

        // Then
        let kept_boxes: Vec<BoxRect> = boxes(&mut kept).into_iter().map(|(_, r)| r).collect();
        let fresh_boxes: Vec<BoxRect> = boxes(&mut fresh).into_iter().map(|(_, r)| r).collect();
        assert_eq!(kept_boxes, fresh_boxes, "after change {step}");
    }
}
