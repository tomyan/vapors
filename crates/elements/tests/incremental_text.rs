//! Parley layouts are kept between updates: a run whose text, styles
//! and width are unchanged is not laid out again (F2c S7).

mod support;

use support::{document, element, text, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;

/// Ten paragraphs and a counter's text node, updated once.
fn page() -> (Document, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let counter = text(&mut doc, body, "tick 0");
    for index in 0..10 {
        let p = element(&mut doc, body, "p", "");
        text(
            &mut doc,
            p,
            &format!("Paragraph {index} has a few words in it."),
        );
    }
    doc.update(viewport(400.0, 600.0)).expect("updates");
    (doc, counter)
}

#[test]
fn an_update_with_nothing_changed_lays_out_no_text() {
    // Given
    let (mut doc, _counter) = page();

    // When
    doc.update(viewport(400.0, 600.0)).expect("updates");

    // Then
    assert_eq!(doc.text_layouts_built(), 0);
}

#[test]
fn a_changed_text_lays_out_only_its_own_run() {
    // Given
    let (mut doc, counter) = page();

    // When
    doc.set_text(counter, "tick 1");
    doc.update(viewport(400.0, 600.0)).expect("updates");

    // Then: the counter's run, at the widths layout and paint ask for
    assert_eq!(doc.text_layouts_built(), COUNTER_LAYOUTS);
}

/// Measured: Taffy measures the run at the available width and paint
/// lays it out at its box's width, the same here, so one layout.
const COUNTER_LAYOUTS: usize = 1;
