//! A number field's box and marks (3-E3c): its text is its content,
//! so its height is its line; a caret after the text while focused by
//! a click, a selection under the whole text when focused by keyboard,
//! neither when unfocused; the caret and a selection where the keys put
//! them; a field without a width is refused.

mod support;

use support::number_page::{click, key, page, shift_key};
use support::{document, element, refusals};
use vapors_dom::Dom;
use vapors_scene::{Color, Item};

/// The quads that are not the field's own white box, as (rect, colour).
fn marks(page: &support::number_page::Page) -> Vec<(vapors_scene::Rect, Color)> {
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
fn a_caret_follows_the_text_when_focused_by_a_click_and_none_unfocused() {
    // Given
    let page = page();
    let unfocused = marks(&page);

    // When
    click(&page, [50.0, 12.0]);
    let caret = marks(&page);

    // Then: one 1 px bar in the text colour, after "20" (about 17 px
    // of 14 px Geist) from the content box's x of 3
    assert!(unfocused.is_empty(), "{unfocused:?}");
    let [(rect, color)] = caret[..] else {
        panic!("one caret, got {caret:?}");
    };
    assert_eq!(color, Color::from_rgba8(0, 0, 0, 255));
    assert_eq!(rect.right - rect.left, 1.0);
    assert!(rect.left > 15.0 && rect.left < 25.0, "{rect:?}");
}

#[test]
fn a_selection_covers_the_text_when_focused_by_keyboard() {
    // Given
    let page = page();

    // When
    key(&page, "Tab");
    let selection = marks(&page);

    // Then: a translucent blue over the text's width
    let [(rect, color)] = selection[..] else {
        panic!("one selection, got {selection:?}");
    };
    assert!((color.a - 0.3).abs() < 0.01 && color.b > 0.8, "{color:?}");
    assert_eq!(rect.left, 3.0);
    assert!(rect.right > 15.0 && rect.right < 25.0, "{rect:?}");
}

#[test]
fn the_caret_and_a_selection_are_painted_where_the_keys_put_them() {
    // Given: "20" focused by a click, the caret after the 0
    let page = page();
    click(&page, [50.0, 12.0]);
    let at_end = marks(&page)[0].0;

    // When
    key(&page, "ArrowLeft");
    let caret = marks(&page);
    shift_key(&page, "ArrowRight");
    let selection = marks(&page);

    // Then: the caret between the digits, about 8 px left of the end;
    // the selection over the 0 alone
    let [(bar, _)] = caret[..] else {
        panic!("one caret, got {caret:?}");
    };
    assert!(at_end.left - bar.left > 5.0, "{bar:?} {at_end:?}");
    let [(over, _)] = selection[..] else {
        panic!("one selection, got {selection:?}");
    };
    assert_eq!((over.left, over.right), (bar.left, at_end.left));
}

#[test]
fn a_field_without_a_width_is_refused_naming_the_fix() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let input = element(&mut doc, body, "input", "");
    doc.set_attribute(input, "type", "number");
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
