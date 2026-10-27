//! A checkbox (3-E3b): a click toggles it after the click's listeners,
//! firing `input` and `change` with `checked`; Space toggles on keyup
//! and Enter does not; a prevented click leaves it; a disabled one
//! takes nothing; `:checked` follows, by click or by property. The
//! order against Chromium is pinned by the corpus's `checkbox.events`.

mod support;

use support::checkbox_page::{checked, click, key, page};
use vapors_dom::{Dom, Event, PropertyValue};
use vapors_elements::Viewport;

const VIEWPORT: Viewport = Viewport {
    width: 100.0,
    height: 100.0,
    scale: 1.0,
};

#[test]
fn a_click_toggles_and_fires_input_then_change_with_the_new_state() {
    // Given
    let page = page();

    // When
    click(&page, [5.0, 5.0]);
    click(&page, [5.0, 18.0]);

    // Then
    assert!(checked(&page, page.visible));
    assert!(!checked(&page, page.hidden));
    assert_eq!(
        *page.log.borrow(),
        [
            "visible input true",
            "visible change true",
            "hidden input false",
            "hidden change false"
        ]
    );
}

#[test]
fn the_toggle_happens_after_the_clicks_listeners_and_a_prevented_click_leaves_it() {
    // Given: a click listener that sees the state and prevents the second click
    let page = page();
    let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let (into, doc) = (std::rc::Rc::clone(&seen), std::rc::Rc::clone(&page.doc));
    let node = page.visible;
    page.doc.borrow_mut().listen(
        node,
        "click",
        Box::new(move |e: &mut Event| {
            let state = doc.borrow().checked(node);
            into.borrow_mut().push(state);
            if into.borrow().len() == 2 {
                e.prevent_default();
            }
        }),
    );

    // When
    click(&page, [5.0, 5.0]);
    click(&page, [5.0, 5.0]);

    // Then: the listener saw the old state each time; the second stayed
    assert_eq!(*seen.borrow(), [Some(false), Some(true)]);
    assert!(checked(&page, page.visible));
    assert_eq!(page.log.borrow().len(), 2, "{:?}", page.log.borrow());
}

#[test]
fn space_toggles_a_focused_checkbox_and_enter_does_not() {
    // Given: focused by Tab
    let page = page();
    key(&page, "Tab");

    // When
    key(&page, "Enter");
    let after_enter = checked(&page, page.visible);
    key(&page, " ");

    // Then
    assert!(!after_enter);
    assert!(checked(&page, page.visible));
    assert_eq!(
        *page.log.borrow(),
        ["visible input true", "visible change true"]
    );
}

#[test]
fn a_disabled_checkbox_takes_no_click_and_no_focus() {
    // Given
    let page = page();

    // When
    click(&page, [5.0, 30.0]);

    // Then
    assert!(!checked(&page, page.off));
    assert_eq!(page.doc.borrow().active_element(), None);
}

#[test]
fn checked_matches_by_click_and_by_property_restyling() {
    // Given: a :checked rule
    let page = page();
    page.doc
        .borrow_mut()
        .add_stylesheet(
            "checked.css",
            "input:checked { background-color: #ff0000; }",
        )
        .expect("parses");
    let red = |page: &support::checkbox_page::Page, node| {
        let mut doc = page.doc.borrow_mut();
        let painted = doc.update(VIEWPORT).expect("updates");
        let rect = painted.border_box(node).expect("a box");
        painted
            .scene()
            .items()
            .iter()
            .any(|item| matches!(item, vapors_scene::Item::Quad(q) if (q.rect.top - rect.y).abs() < 0.5 && q.background == vapors_scene::Color::from_rgba8(255, 0, 0, 255)))
    };
    let before = (red(&page, page.visible), red(&page, page.hidden));

    // When: visible clicked on, hidden set off by property
    click(&page, [5.0, 5.0]);
    page.doc
        .borrow_mut()
        .set_property(page.hidden, "checked", PropertyValue::Bool(false));

    // Then
    assert_eq!(before, (false, true));
    assert_eq!(
        (red(&page, page.visible), red(&page, page.hidden)),
        (true, false)
    );
    assert_eq!(page.log.borrow().len(), 2, "a property set fires nothing");
}

#[test]
fn checked_is_none_for_an_element_that_is_not_a_checkbox() {
    // Given
    let page = page();
    let body = page.doc.borrow().body();

    // Then
    assert_eq!(page.doc.borrow().checked(body), None);
}
