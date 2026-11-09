//! A press reaches the node a person would see (R1): a node outside
//! its scroll container's view is scrolled into it first, as Chromium's
//! accessibility default action does; one still outside the viewport,
//! or with `pointer-events: none`, is refused as its own problem.

mod support;

use support::drive::{log, node, shared, target};
use support::roles_page::{button, labelled, page};
use support::{document, viewport};
use vapors_elements::{AxError, Document};

/// A list 48 px tall scrolling six buttons 24 px tall, `B1` to `B6`.
fn list_page() -> (Document, vapors_dom::NodeId) {
    let mut doc = document();
    let body = doc.body();
    let list = labelled(&mut doc, body, "div", &[("id", "list")]);
    for n in 1..=6 {
        button(&mut doc, list, &format!("B{n}"), &[]);
    }
    let style = "body { margin: 0; } \
        #list { overflow: auto; height: 48px; width: 100px; } \
        button { display: block; box-sizing: border-box; width: 80px; height: 24px; \
        margin: 0; padding: 0; }";
    doc.add_stylesheet("list.css", style).expect("parses");
    doc.update(viewport(640.0, 400.0)).expect("updates");
    (doc, list)
}

#[test]
fn a_press_scrolls_a_node_outside_its_container_into_view_then_clicks() {
    // Given: B6 sits 120 px down a list showing 48
    let (page, list) = list_page();
    let doc = shared(page);
    let sixth = node(&doc, "button", "B6");
    let events = log(&doc, sixth, &["click"]);

    // When
    Document::ax_press(&doc, &target("button", "B6")).expect("pressed");

    // Then: centring B6 asks 108 px, the list goes as far as 96
    assert_eq!(*events.borrow(), ["click"]);
    assert_eq!(doc.borrow().scroll_offset(list), Some([0.0, 96.0]));
}

#[test]
fn a_press_on_a_node_wholly_in_view_scrolls_nothing() {
    // Given: B2 fills the list's lower half; centring it would scroll 12
    let (page, list) = list_page();
    let doc = shared(page);

    // When
    Document::ax_press(&doc, &target("button", "B2")).expect("pressed");

    // Then
    assert_eq!(doc.borrow().scroll_offset(list), Some([0.0, 0.0]));
}

#[test]
fn a_press_on_a_node_outside_the_viewport_is_refused_by_name() {
    // Given: a button below the 400 px viewport, in no scroll container
    let mut page = page();
    let body = page.body();
    let low = "display: block; position: absolute; left: 0; top: 500px";
    button(&mut page, body, "Low", &[("style", low)]);
    page.update(viewport(640.0, 400.0)).expect("updates");
    let doc = shared(page);

    // When
    let pressed = Document::ax_press(&doc, &target("button", "Low"));

    // Then
    let Err(AxError::OutOfView { role, name, .. }) = pressed else {
        panic!("out of view expected, got {pressed:?}");
    };
    assert_eq!((role.as_str(), name.as_str()), ("button", "Low"));
}

#[test]
fn a_press_on_a_node_with_pointer_events_none_is_refused_as_its_own() {
    // Given: Ghost takes no pointer; its row behind it does
    let mut page = page();
    let dialog = page.children(page.body())[0];
    let row = *page.children(dialog).last().expect("a row");
    button(
        &mut page,
        row,
        "Ghost",
        &[("style", "pointer-events: none")],
    );
    page.update(viewport(640.0, 400.0)).expect("updates");
    let doc = shared(page);

    // When
    let pressed = Document::ax_press(&doc, &target("button", "Ghost"));

    // Then
    let refused = AxError::NoPointer {
        role: "button".to_owned(),
        name: "Ghost".to_owned(),
    };
    assert_eq!(pressed, Err(refused));
}
