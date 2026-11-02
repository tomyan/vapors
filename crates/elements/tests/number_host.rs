//! A host that refuses a committed number puts its own back with
//! `Document::set_committed_number`: the field shows it, nothing fires,
//! and the same number typed again fires `change` again.

mod support;

use support::number_page::{click, key, page, type_text, value};

#[test]
fn after_the_host_puts_its_number_back_the_same_typed_number_changes_again() {
    // Given: 205 typed and committed, over the max, and refused
    let page = page();
    click(&page, [50.0, 12.0]);
    type_text(&page, "5");
    key(&page, "Enter");

    // When: the host shows its 20 again, then the person types 5 again
    let set = page
        .doc
        .borrow_mut()
        .set_committed_number(page.amount, 20.0);
    let shown = value(&page, page.amount);
    type_text(&page, "5");
    key(&page, "Enter");

    // Then
    assert!(set);
    assert_eq!(shown, "20");
    assert_eq!(
        *page.log.borrow(),
        [
            "amount input \"205\" 205",
            "amount change \"205\" 205",
            "amount input \"205\" 205",
            "amount change \"205\" 205"
        ]
    );
}

#[test]
fn set_committed_number_on_anything_but_a_number_field_says_so() {
    // Given
    let page = page();
    let body = page.doc.borrow().body();

    // When
    let set = page.doc.borrow_mut().set_committed_number(body, 20.0);

    // Then
    assert!(!set);
}
