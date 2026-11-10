//! A setter given the value the node already holds: what it marks
//! changed and what the next update redoes. Characterisation of today's
//! behaviour, recorded in docs/design/incremental-contracts.md.

mod support;

use support::{document, element, text, viewport};
use vapors_dom::{Dom, NodeId, PropertyValue};
use vapors_elements::Document;

const SHEET: &str =
    ".cell { color: red; } input { display: block; } input:checked { color: blue; }";

struct Page {
    doc: Document,
    cell: NodeId,
    label: NodeId,
    checkbox: NodeId,
}

/// What one update after a write did.
#[derive(Debug, PartialEq)]
struct Work {
    changed: bool,
    restyled: usize,
    laid_out: usize,
    text_layouts: usize,
}

/// Ten cells with text, the first with an element child as well, and a
/// checked checkbox, updated once, with the change flag taken.
fn page() -> Page {
    let mut doc = document();
    let body = doc.body();
    let mut cells = (0..10).map(|index| {
        let cell = element(&mut doc, body, "div", "cell");
        (cell, text(&mut doc, cell, &format!("cell {index}")))
    });
    let (cell, label) = cells.next().expect("ten cells");
    cells.for_each(drop);
    element(&mut doc, cell, "span", "");
    let checkbox = element(&mut doc, body, "input", "");
    doc.set_attribute(checkbox, "type", "checkbox");
    doc.set_property(checkbox, "checked", PropertyValue::Bool(true));
    doc.set_property(checkbox, "disabled", PropertyValue::Bool(false));
    doc.add_stylesheet("app.css", SHEET).expect("parses");
    doc.update(viewport(400.0, 600.0)).expect("updates");
    doc.take_changed();
    Page {
        doc,
        cell,
        label,
        checkbox,
    }
}

fn work_after(write: impl FnOnce(&mut Page)) -> Work {
    let mut page = page();
    write(&mut page);
    let changed = page.doc.take_changed();
    page.doc.update(viewport(400.0, 600.0)).expect("updates");
    Work {
        changed,
        restyled: page.doc.elements_restyled(),
        laid_out: page.doc.boxes_laid_out(),
        text_layouts: page.doc.text_layouts_built(),
    }
}

fn work(changed: bool, restyled: usize, laid_out: usize) -> Work {
    Work {
        changed,
        restyled,
        laid_out,
        text_layouts: 0,
    }
}

#[test]
fn with_no_write_nothing_is_changed_or_redone() {
    // When
    let none = work_after(|_| {});

    // Then
    assert_eq!(none, work(false, 0, 0));
}

#[test]
fn the_same_text_marks_the_document_changed_and_redoes_no_work() {
    // When
    let same = work_after(|p| p.doc.set_text(p.label, "cell 0"));

    // Then
    assert_eq!(same, work(true, 0, 0));
}

#[test]
fn the_same_attribute_marks_changed_and_restyles_the_elements_subtree() {
    // When
    let same = work_after(|p| p.doc.set_attribute(p.cell, "class", "cell"));

    // Then: the cell and its span child
    assert_eq!(same, work(true, 2, 0));
}

#[test]
fn the_same_checked_property_marks_changed_and_restyles_the_element() {
    // When
    let same = work_after(|p| {
        p.doc
            .set_property(p.checkbox, "checked", PropertyValue::Bool(true));
    });

    // Then
    assert_eq!(same, work(true, 1, 0));
}

#[test]
fn the_same_other_property_marks_changed_and_restyles_nothing() {
    // When
    let same = work_after(|p| {
        p.doc
            .set_property(p.checkbox, "disabled", PropertyValue::Bool(false));
    });

    // Then
    assert_eq!(same, work(true, 0, 0));
}
