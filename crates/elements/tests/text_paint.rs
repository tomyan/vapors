//! A text field's marks (E6a): a caret after the text when focused by a
//! click at its end, a selection over the whole text when focused by
//! keyboard, nothing unfocused, the caret following a restyle, composing
//! text underlined; a field
//! without a width is refused.

mod support;

use support::text_page::{click, key, page};
use support::{document, element, refusals};
use vapors_scene::{Color, Item};

/// The quads over the first field that are not its own white box.
fn marks(page: &support::text_page::Page) -> Vec<(vapors_scene::Rect, Color)> {
    let mut doc = page.doc.borrow_mut();
    let painted = doc
        .update(support::viewport(400.0, 100.0))
        .expect("updates");
    painted
        .scene()
        .items()
        .iter()
        .filter_map(|item| match item {
            Item::Quad(q)
                if q.rect.top < 24.0
                    && q.background.a > 0.0
                    && q.background != Color::from_rgba8(255, 255, 255, 255) =>
            {
                Some((q.rect, q.background))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn a_caret_stands_after_the_text_when_focused_by_a_click_at_its_end() {
    // Given
    let page = page();
    let unfocused = marks(&page);

    // When
    click(&page, [120.0, 12.0]);
    let caret = marks(&page);

    // Then: one 1 px bar in the text colour, after "Warm" (about 38 px
    // of 14 px Geist) from the content box's x of 3
    assert!(unfocused.is_empty(), "{unfocused:?}");
    let [(rect, color)] = caret[..] else {
        panic!("one caret, got {caret:?}");
    };
    assert_eq!(color, Color::from_rgba8(0, 0, 0, 255));
    assert_eq!(rect.right - rect.left, 1.0);
    assert!(rect.left > 35.0 && rect.left < 48.0, "{rect:?}");
}

#[test]
fn the_caret_follows_the_text_when_the_field_is_restyled() {
    // Given: the caret after "Warm" at 14 px
    let page = page();
    click(&page, [120.0, 12.0]);
    let small = marks(&page);

    // When: the text doubles in size
    page.doc
        .borrow_mut()
        .add_stylesheet("big.css", "input { font-size: 28px; }")
        .expect("parses");
    let large = marks(&page);

    // Then: the caret stands after the larger text, about twice as far
    let ([(before, _)], [(after, _)]) = (&small[..], &large[..]) else {
        panic!("one caret each, got {small:?} and {large:?}");
    };
    assert!(after.left > before.left * 1.6, "{before:?} then {after:?}");
}

#[test]
fn composing_text_is_underlined_in_the_text_colour() {
    // Given: an IME composing "th" after "Warm"
    let page = page();
    click(&page, [120.0, 12.0]);
    let edit = vapors_elements::TextEdit {
        before: "Warm".into(),
        text: "Warmth".into(),
        selection: (6, 6),
        composing: Some((4, 6)),
    };
    vapors_elements::Document::text_edited(&page.doc, &edit).expect("applies");

    // When
    let marks = marks(&page);

    // Then: a 1 px line under "th", beside the caret
    let lines: Vec<_> = marks
        .iter()
        .filter(|(rect, _)| {
            (rect.bottom - rect.top - 1.0).abs() < 0.01 && rect.right - rect.left > 4.0
        })
        .collect();
    let [(line, color)] = lines[..] else {
        panic!("one underline, got {marks:?}");
    };
    assert_eq!(*color, Color::from_rgba8(0, 0, 0, 255));
    assert!(
        line.left > 35.0 && line.right < 60.0 && line.top > 12.0,
        "{line:?}"
    );
}

#[test]
fn a_selection_covers_the_text_when_focused_by_keyboard() {
    // Given
    let page = page();

    // When
    key(&page, "Tab");
    let selection = marks(&page);

    // Then: a translucent blue from the text's start over its width
    let [(rect, color)] = selection[..] else {
        panic!("one selection, got {selection:?}");
    };
    assert!((color.a - 0.3).abs() < 0.01 && color.b > 0.8, "{color:?}");
    assert!(rect.left >= 3.0 && rect.left < 5.0, "{rect:?}");
    assert!(rect.right > 35.0 && rect.right < 48.0, "{rect:?}");
}

#[test]
fn a_text_field_without_a_width_is_refused_naming_the_fix() {
    // Given: no type is a text field
    let mut doc = document();
    let body = doc.body();
    element(&mut doc, body, "input", "");
    doc.add_stylesheet("app.css", "input { display: block; }")
        .expect("parses");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("`width: auto`") && errors[0].contains("give it a width"),
        "{errors:?}"
    );
}
