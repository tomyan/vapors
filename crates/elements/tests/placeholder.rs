//! An empty text field's placeholder (`docs/placeholder.md`): painted
//! in the field's font and Chromium's grey, from the left
//! of the content box, centred in it and cut to it; gone while the
//! field has a value, however it got one; never a hit or a selection.

use crate::support;

use support::text_page::{Page, click, key, page, type_text};
use vapors_dom::{Dom, PropertyValue};
use vapors_scene::{Color, Item, Rect};

/// The placeholder-bearing field: the page's second, y 24 to 48, its
/// content box x 3 to 157, y 26 to 46 (the user agent's 1 px border,
/// the page's padding of 1 px 2 px).
fn with_placeholder(text: &str) -> Page {
    let page = page();
    page.doc
        .borrow_mut()
        .set_attribute(page.plain, "placeholder", text);
    page
}

/// What the page paints for text: each run's string, ink colour,
/// size, origin, clip and line height.
type Painted = (String, Color, f32, [f32; 2], Option<Rect>, f32);

fn texts(page: &Page) -> Vec<Painted> {
    let mut doc = page.doc.borrow_mut();
    let painted = doc
        .update(support::viewport(400.0, 100.0))
        .expect("updates");
    let scene = painted.scene().to_scene();
    let mut found = Vec::new();
    for (item, clip) in scene.items().iter().zip(scene.clips()) {
        if let Item::Text { text, origin, .. } = item {
            let layout = painted.text(*text);
            let run = &layout.runs()[0];
            found.push((
                layout.text().to_owned(),
                run.color,
                run.size,
                *origin,
                *clip,
                layout.height(),
            ));
        }
    }
    found
}

fn shown(page: &Page) -> Vec<Painted> {
    texts(page)
        .into_iter()
        .filter(|t| t.0 == "Search")
        .collect()
}

#[test]
fn an_empty_field_paints_its_placeholder_in_grey_from_the_content_box_clipped_to_it() {
    // Given: a 14 px field, 24 px tall with 1 px padding
    let page = with_placeholder("Search");

    // When
    let found = shown(&page);

    // Then: one run in Chromium's #757575, the field's size, at the
    // content box's left, centred in its 20 px, cut to it
    let [(_, color, size, origin, clip, line)] = found[..] else {
        panic!("one placeholder, got {found:?}");
    };
    assert_eq!(color, Color::from_rgba8(117, 117, 117, 255));
    assert_eq!(size, 14.0);
    assert_eq!(origin[0], 3.0);
    // the line's top is half of what the content box's 20 px leave
    assert!(line > 14.0 && line < 20.0, "a line of text: {line}");
    assert_eq!(origin[1], 26.0 + (20.0 - line) / 2.0);
    assert_eq!(clip, Some(Rect::new(3.0, 26.0, 157.0, 46.0)));
}

#[test]
fn a_field_with_a_value_paints_none() {
    // Given: the first field has a value, and a placeholder
    let page = page();
    page.doc
        .borrow_mut()
        .set_attribute(page.name, "placeholder", "Search");

    // Then: its value is painted, and no placeholder
    assert_eq!(shown(&page), []);
    assert!(texts(&page).iter().any(|t| t.0 == "Warm"));
}

#[test]
fn a_value_set_by_a_signal_hides_it_and_emptying_the_value_brings_it_back() {
    // Given
    let page = with_placeholder("Search");
    assert_eq!(shown(&page).len(), 1);

    // When: the binding sets a value, then an empty one
    let set = |value: &str| {
        let mut doc = page.doc.borrow_mut();
        doc.set_property(page.plain, "value", PropertyValue::Text(value.into()));
    };
    set("ab");
    let filled = texts(&page).iter().map(|t| t.0.clone()).collect::<Vec<_>>();
    set("");

    // Then
    assert_eq!(filled, ["Warm", "ab", "off"]);
    assert_eq!(shown(&page).len(), 1);
}

#[test]
fn typing_hides_it_and_deleting_the_text_shows_it_again() {
    // Given: the empty field focused
    let page = with_placeholder("Search");
    click(&page, [80.0, 36.0]);

    // When
    type_text(&page, "a");
    let typed = shown(&page).len();
    key(&page, "Backspace");

    // Then
    assert_eq!((typed, shown(&page).len()), (0, 1));
}

#[test]
fn it_is_no_hit_and_no_selection() {
    // Given: a click on the placeholder's text
    let page = with_placeholder("Search");
    click(&page, [20.0, 36.0]);

    // Then: the field itself is hit, holds no text and selects none
    let doc = page.doc.borrow();
    assert_eq!(doc.value(page.plain).as_deref(), Some(""));
    assert_eq!(doc.selection(page.plain), Some((0, 0)));
}
