//! A number field's state (3-E3c): `:invalid` while its text is no
//! number or out of range, following a later `max`; its editing state
//! going with it when removed; a scrub counting from the last committed
//! number when the text is none; a fractional step written in decimal.

mod support;

use support::number_page::{click, key, page, page_with, type_text, value};
use vapors_dom::Dom;
use vapors_elements::Document;

/// Whether the `amount` field, at the top, is painted red by an
/// `:invalid` rule.
fn red(page: &support::number_page::Page) -> bool {
    let mut doc = page.doc.borrow_mut();
    let painted = doc
        .update(support::viewport(400.0, 100.0))
        .expect("updates");
    painted.scene().items().iter().any(|item| {
        matches!(item, vapors_scene::Item::Quad(q) if q.rect.top == 0.0 && q.background == vapors_scene::Color::from_rgba8(255, 0, 0, 255))
    })
}

fn with_invalid_rule(page: &support::number_page::Page) {
    let rule = "input:invalid { background-color: #ff0000; }";
    page.doc
        .borrow_mut()
        .add_stylesheet("invalid.css", rule)
        .expect("parses");
}

#[test]
fn invalid_follows_a_max_narrowed_under_the_value() {
    // Given: an :invalid rule and a committed 20
    let page = page();
    with_invalid_rule(&page);
    assert!(!red(&page));

    // When: max drops under it
    page.doc
        .borrow_mut()
        .set_attribute(page.amount, "max", "10");

    // Then
    assert!(red(&page));
}

#[test]
fn a_removed_field_takes_its_editing_state_with_it() {
    // Given: focused by Tab (selected), then removed
    let page = page();
    key(&page, "Tab");

    // When
    page.doc.borrow_mut().remove(page.amount);

    // Then: nothing selected, nothing focused
    assert_eq!(page.doc.borrow().active_element(), None);
    assert!(!page.doc.borrow().text_selected(page.amount));
}

#[test]
fn a_scrub_that_starts_on_text_that_is_no_number_counts_from_the_last_committed() {
    // Given: `scrub` focused by a click, "50e" typed, no number
    let page = page();
    click(&page, [50.0, 36.0]);
    type_text(&page, "e");

    // When: dragged 4 px right, two steps
    Document::pointer_pressed(&page.doc, [50.0, 36.0]);
    Document::pointer_moved(&page.doc, [54.0, 36.0]);
    Document::pointer_released(&page.doc, [54.0, 36.0]);

    // Then: from 50, not from 0
    assert_eq!(value(&page, page.scrub), "52");
}

#[test]
fn a_fractional_step_writes_decimal_text_not_f64_noise() {
    // Given: 0..1 by 0.1 at 0.2
    let page = page_with(&[
        ("min", "0"),
        ("max", "1"),
        ("step", "0.1"),
        ("value", "0.2"),
    ]);
    click(&page, [50.0, 12.0]);

    // When
    key(&page, "ArrowUp");

    // Then
    assert_eq!(value(&page, page.amount), "0.3");
    assert_eq!(
        *page.log.borrow(),
        ["amount input \"0.3\" 0.3", "amount change \"0.3\" 0.3"]
    );
}

#[test]
fn invalid_matches_while_the_text_is_no_number_or_out_of_range() {
    // Given: an :invalid rule
    let page = page();
    with_invalid_rule(&page);
    click(&page, [50.0, 12.0]);

    // When / Then
    assert!(!red(&page));
    type_text(&page, "5");
    assert!(red(&page), "205 is over 100");
    key(&page, "Backspace");
    type_text(&page, "e");
    assert!(red(&page), "20e is no number");
    key(&page, "Enter");
    assert!(!red(&page), "reverted to 20");
}
