//! A hidden labelling element's text changing renames the element it
//! labels (the #432 review's L4).

use crate::support;

use support::mirror::{labels, node_of, step, taken_model};
use support::{element, text};
use vapors_dom::Dom;

#[test]
fn a_hidden_labels_text_changing_renames_the_labelled_button() {
    // Given: After labelled by a hidden span reading "Later"
    let (doc, mut model) = taken_model();
    let after = node_of(&doc, "button", "After");
    let row = doc.borrow().parent(after).expect("the row");
    let words = {
        let mut doc = doc.borrow_mut();
        doc.set_attribute(after, "aria-labelledby", "late");
        let late = element(&mut doc, row, "span", "");
        doc.set_attribute(late, "id", "late");
        doc.set_attribute(late, "hidden", "");
        text(&mut doc, late, "Later")
    };
    step(&doc, &mut model).expect("named");

    // When: the span's text changes
    doc.borrow_mut().set_text(words, "Sooner");
    let update = step(&doc, &mut model);

    // Then
    let nodes = update
        .map(|update| labels(&update.nodes))
        .unwrap_or_default();
    assert!(nodes.contains(&"button \"Sooner\"".to_owned()), "{nodes:?}");
}
