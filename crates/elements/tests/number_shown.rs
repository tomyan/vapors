//! A number field shows a value set as a number at its step's precision
//! (`number.rs` `shown`), on the painted page; its DOM value keeps the
//! full number. Chromium paints the full number; this differs on
//! purpose, for display only (touché, ux-review-1).

mod support;

use support::number_page::{page, value};
use support::viewport;
use vapors_dom::{Dom, PropertyValue};
use vapors_scene::Item;

#[test]
fn a_number_set_on_a_field_by_1_paints_whole_and_keeps_its_value() {
    // Given: the field steps by 1
    let page = page();

    // When
    page.doc
        .borrow_mut()
        .set_property(page.amount, "value", PropertyValue::Number(7.1465));
    page.doc
        .borrow_mut()
        .update(viewport(400.0, 100.0))
        .expect("updates");

    // Then
    let doc = page.doc.borrow();
    let painted = doc.painted().expect("painted");
    let texts: Vec<String> = painted
        .scene()
        .items()
        .iter()
        .filter_map(|item| match item {
            Item::Text { text, .. } => Some(painted.text(*text).text().to_owned()),
            _ => None,
        })
        .collect();
    assert!(texts.iter().any(|t| t == "7"), "{texts:?}");
    assert!(!texts.iter().any(|t| t.contains("7.1465")), "{texts:?}");
    drop(doc);
    assert_eq!(value(&page, page.amount), "7.1465");
}
