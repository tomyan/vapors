//! A number field's editing (3-E3c): typed characters a number takes
//! go to the end, letters do not; Backspace deletes; Enter and blur
//! commit, clamping an out-of-range number and reverting text that is
//! no number; arrows step, Shift by ten; keyboard focus selects the
//! text whole, so typing replaces it. The order against Chromium is
//! pinned by the corpus's `number.events`; these pin the values and
//! vapors' own commit rule.

mod support;

use support::number_page::{click, key, page, shift_key, type_text, value};
use vapors_dom::{Dom, PropertyValue};

#[test]
fn typed_digits_go_to_the_end_and_letters_are_dropped() {
    // Given: focused by a click, the caret at the end
    let page = page();
    click(&page, [50.0, 12.0]);

    // When
    type_text(&page, "5");
    type_text(&page, "x");
    key(&page, "Backspace");

    // Then
    assert_eq!(
        *page.log.borrow(),
        ["amount input \"205\" 205", "amount input \"20\" 20"]
    );
}

#[test]
fn text_that_is_no_number_has_an_empty_value_and_no_number_until_committed() {
    // Given
    let page = page();
    click(&page, [50.0, 12.0]);

    // When: "20e" is no number; Enter reverts it to the committed 20
    type_text(&page, "e");
    let while_editing = value(&page, page.amount);
    key(&page, "Enter");

    // Then
    assert_eq!(while_editing, "");
    assert_eq!(value(&page, page.amount), "20");
    assert_eq!(
        *page.log.borrow(),
        ["amount input \"\" none"],
        "the revert fires no input and no change"
    );
}

#[test]
fn enter_commits_an_out_of_range_number_clamped_with_one_change() {
    // Given
    let page = page();
    click(&page, [50.0, 12.0]);

    // When: 205 is over 100
    type_text(&page, "5");
    key(&page, "Enter");
    key(&page, "Enter");

    // Then: clamped to 100, one change
    assert_eq!(value(&page, page.amount), "100");
    assert_eq!(
        *page.log.borrow(),
        ["amount input \"205\" 205", "amount change \"100\" 100"]
    );
}

#[test]
fn blur_commits_too_before_the_focus_leaves() {
    // Given: typed 7 after 20
    let page = page();
    click(&page, [50.0, 12.0]);
    type_text(&page, "7");
    let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let into = std::rc::Rc::clone(&seen);
    page.doc.borrow_mut().listen(
        page.amount,
        "blur",
        Box::new(move |_: &mut vapors_dom::Event| into.borrow_mut().push("blur")),
    );
    let into = std::rc::Rc::clone(&seen);
    page.doc.borrow_mut().listen(
        page.amount,
        "change",
        Box::new(move |_: &mut vapors_dom::Event| into.borrow_mut().push("change")),
    );

    // When: a click on the disabled field blurs amount
    click(&page, [50.0, 60.0]);

    // Then
    assert_eq!(*seen.borrow(), ["change", "blur"]);
    assert_eq!(value(&page, page.amount), "100");
}

#[test]
fn arrows_step_by_one_and_shift_by_ten_with_input_and_change_each() {
    // Given
    let page = page();
    click(&page, [50.0, 12.0]);

    // When
    key(&page, "ArrowUp");
    shift_key(&page, "ArrowDown");
    key(&page, "ArrowDown");

    // Then: 21, 11, 10
    assert_eq!(
        *page.log.borrow(),
        [
            "amount input \"21\" 21",
            "amount change \"21\" 21",
            "amount input \"11\" 11",
            "amount change \"11\" 11",
            "amount input \"10\" 10",
            "amount change \"10\" 10"
        ]
    );
}

#[test]
fn keyboard_focus_selects_the_text_so_typing_replaces_it_and_a_click_does_not() {
    // Given: Tab focuses amount
    let page = page();
    key(&page, "Tab");
    let selected = page.doc.borrow().text_selected(page.amount);

    // When
    type_text(&page, "9");
    let replaced = value(&page, page.amount);
    click(&page, [50.0, 12.0]);
    type_text(&page, "1");

    // Then
    assert!(selected);
    assert_eq!(replaced, "9");
    assert_eq!(value(&page, page.amount), "91");
}

#[test]
fn a_disabled_field_takes_no_text_and_no_focus() {
    // Given
    let page = page();

    // When
    click(&page, [50.0, 60.0]);
    type_text(&page, "1");

    // Then
    assert_eq!(value(&page, page.off), "5");
    assert_eq!(page.doc.borrow().active_element(), None);
}

#[test]
fn a_value_property_sets_the_text_and_fires_nothing() {
    // Given
    let page = page();

    // When
    page.doc
        .borrow_mut()
        .set_property(page.amount, "value", PropertyValue::Number(42.5));

    // Then
    assert_eq!(value(&page, page.amount), "42.5");
    assert!(page.log.borrow().is_empty());
}

#[test]
fn invalid_matches_while_the_text_is_no_number_or_out_of_range() {
    // Given: an :invalid rule
    let page = page();
    page.doc
        .borrow_mut()
        .add_stylesheet(
            "invalid.css",
            "input:invalid { background-color: #ff0000; }",
        )
        .expect("parses");
    let red = |page: &support::number_page::Page| {
        let mut doc = page.doc.borrow_mut();
        let painted = doc
            .update(support::viewport(400.0, 100.0))
            .expect("updates");
        // :invalid rule's quad: the field's box at the top, red
        painted.scene().items().iter().any(|item| {
            matches!(item, vapors_scene::Item::Quad(q) if q.rect.top == 0.0 && q.background == vapors_scene::Color::from_rgba8(255, 0, 0, 255))
        })
    };
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
