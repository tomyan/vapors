//! A text field by click count (E6a): a double click selects the word
//! after the caret position nearest it (the space after a word just
//! past its end, the last word past the text) and typing replaces the
//! word; a triple click selects the whole text; a double click starts
//! no drag; a press inside the selection places the caret on release. The platform counts the
//! clicks (`vapors-app`'s `Clicks`); Chromium's answers for the same
//! presses are pinned by the corpus's `words.events`.

mod support;

use std::num::NonZeroU32;

use support::text_page::{page, selection, type_text, value};
use vapors_dom::{Dom, PropertyValue};
use vapors_elements::Document;

/// "Warm tones" in the first field: "tones" spans about x 45..80.
fn warm_tones() -> support::text_page::Page {
    let page = page();
    page.doc.borrow_mut().set_property(
        page.name,
        "value",
        PropertyValue::Text("Warm tones".into()),
    );
    page.doc
        .borrow_mut()
        .update(support::viewport(400.0, 100.0))
        .expect("updates");
    page
}

/// Press and release at `point`, the press the `clicks`th in a row.
fn click_nth(page: &support::text_page::Page, point: [f32; 2], clicks: u32) {
    let clicks = NonZeroU32::new(clicks).expect("a press is at least the first");
    Document::pointer_pressed_clicks(&page.doc, point, clicks);
    Document::pointer_released(&page.doc, point);
}

#[test]
fn a_double_click_selects_the_word_under_it() {
    // Given
    let page = warm_tones();

    // When
    click_nth(&page, [60.0, 12.0], 1);
    click_nth(&page, [60.0, 12.0], 2);

    // Then: "tones"
    assert_eq!(selection(&page, page.name), (5, 10));
}

#[test]
fn typing_after_a_double_click_replaces_the_word() {
    // Given: "Warm" double-clicked
    let page = warm_tones();
    click_nth(&page, [15.0, 12.0], 1);
    click_nth(&page, [15.0, 12.0], 2);

    // When
    type_text(&page, "Cool");

    // Then
    assert_eq!(value(&page, page.name), "Cool tones");
}

#[test]
fn a_triple_click_selects_the_whole_text() {
    // Given
    let page = warm_tones();

    // When
    for clicks in 1..=3 {
        click_nth(&page, [60.0, 12.0], clicks);
    }

    // Then
    assert_eq!(selection(&page, page.name), (0, 10));
}

#[test]
fn a_double_click_starts_no_drag() {
    // Given: "tones" double-clicked, the button held
    let page = warm_tones();
    click_nth(&page, [60.0, 12.0], 1);
    let second = NonZeroU32::new(2).expect("two");
    Document::pointer_pressed_clicks(&page.doc, [60.0, 12.0], second);

    // When: the pointer moves to the start before release
    Document::pointer_moved(&page.doc, [3.0, 12.0]);
    Document::pointer_released(&page.doc, [3.0, 12.0]);

    // Then: still the word
    assert_eq!(selection(&page, page.name), (5, 10));
}

#[test]
fn a_double_click_just_past_a_word_selects_the_space_after_it() {
    // Given: "Warm" ends near x 38, "tones" starts near x 42
    let page = warm_tones();

    // When
    click_nth(&page, [39.0, 12.0], 1);
    click_nth(&page, [39.0, 12.0], 2);

    // Then
    assert_eq!(selection(&page, page.name), (4, 5));
}

#[test]
fn a_double_click_past_the_text_selects_the_last_word() {
    // Given
    let page = warm_tones();

    // When
    click_nth(&page, [150.0, 12.0], 1);
    click_nth(&page, [150.0, 12.0], 2);

    // Then
    assert_eq!(selection(&page, page.name), (5, 10));
}

#[test]
fn a_press_inside_the_selection_places_the_caret_on_release() {
    // Given: "tones" selected
    let page = warm_tones();
    click_nth(&page, [60.0, 12.0], 1);
    click_nth(&page, [60.0, 12.0], 2);

    // When
    Document::pointer_pressed_clicks(&page.doc, [70.0, 12.0], NonZeroU32::MIN);
    let pressed = selection(&page, page.name);
    Document::pointer_released(&page.doc, [70.0, 12.0]);

    // Then: the selection held through the press, then a caret in "tones"
    assert_eq!(pressed, (5, 10));
    let (start, end) = selection(&page, page.name);
    assert!(start == end && (6..10).contains(&start), "{start}..{end}");
}

#[test]
fn a_click_listener_reads_the_selection_held_until_after_the_click() {
    // Given: "tones" selected, and a click listener noting the selection
    let page = warm_tones();
    click_nth(&page, [60.0, 12.0], 1);
    click_nth(&page, [60.0, 12.0], 2);
    let seen = std::rc::Rc::new(std::cell::RefCell::new(None));
    let into = std::rc::Rc::clone(&seen);
    let doc = std::rc::Rc::clone(&page.doc);
    let field = page.name;
    page.doc.borrow_mut().listen(
        page.name,
        "click",
        Box::new(move |_: &mut vapors_dom::Event| {
            *into.borrow_mut() = doc.borrow().selection(field);
        }),
    );

    // When: a click inside the selection
    click_nth(&page, [70.0, 12.0], 1);

    // Then: the click saw the selection whole, as Chromium's does; the
    // caret moved after it
    assert_eq!(*seen.borrow(), Some((5, 10)));
    let (start, end) = selection(&page, page.name);
    assert!(start == end && (6..10).contains(&start), "{start}..{end}");
}

#[test]
fn a_double_click_released_then_moved_keeps_the_word() {
    // Given: "tones" double-clicked, the button released
    let page = warm_tones();
    click_nth(&page, [60.0, 12.0], 1);
    click_nth(&page, [60.0, 12.0], 2);

    // When: the pointer moves with no button held
    Document::pointer_moved(&page.doc, [3.0, 12.0]);

    // Then: nothing drags; the word stays selected
    assert_eq!(selection(&page, page.name), (5, 10));
}
