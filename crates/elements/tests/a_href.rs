//! Links (`<a href>`): a click on one, or on what it holds, asks to go
//! to its `href` unless a listener prevented the click; Enter on a
//! focused link does too, Space does not (as in Chromium); an `a`
//! without `href` is no link. The app takes the address
//! (`Document::take_navigation`) and the platform goes there.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::{document, viewport};
use vapors_dom::{Dom, Event, NodeId};
use vapors_elements::{Document, Modifiers};

const SHEET: &str = "body { margin: 0; } a { display: block; height: 20px; }";

struct Page {
    doc: Rc<RefCell<Document>>,
    /// y 0..20, `href="?demo=counter"`, holding a span.
    link: NodeId,
    /// y 20..40, no `href`.
    plain: NodeId,
}

fn page() -> Page {
    let mut doc = document();
    let body = doc.body();
    let link = anchor(&mut doc, body, Some("?demo=counter"), "Counter");
    let plain = anchor(&mut doc, body, None, "Nowhere");
    doc.add_stylesheet("app.css", SHEET).expect("parses");
    doc.update(viewport(200.0, 100.0)).expect("updates");
    Page {
        doc: Rc::new(RefCell::new(doc)),
        link,
        plain,
    }
}

fn anchor(doc: &mut Document, parent: NodeId, href: Option<&str>, label: &str) -> NodeId {
    let a = doc.create_element("a");
    if let Some(href) = href {
        doc.set_attribute(a, "href", href);
    }
    let span = doc.create_element("span");
    let text = doc.create_text(label);
    doc.append(span, text);
    doc.append(a, span);
    doc.append(parent, a);
    a
}

fn click(page: &Page, point: [f32; 2]) {
    Document::pointer_pressed(&page.doc, point);
    Document::pointer_released(&page.doc, point);
}

fn key(page: &Page, key: &str) {
    let none = Modifiers::default();
    Document::key_pressed(&page.doc, key, none);
    Document::key_released(&page.doc, key, none);
}

fn taken(page: &Page) -> Option<String> {
    page.doc.borrow_mut().take_navigation()
}

#[test]
fn a_click_on_what_a_link_holds_asks_to_go_to_its_href_once() {
    // Given
    let page = page();

    // When: on the link's span
    click(&page, [5.0, 5.0]);

    // Then
    assert_eq!(taken(&page).as_deref(), Some("?demo=counter"));
    assert_eq!(taken(&page), None);
}

#[test]
fn a_prevented_click_on_a_link_goes_nowhere() {
    // Given
    let page = page();
    page.doc.borrow_mut().listen(
        page.link,
        "click",
        Box::new(|e: &mut Event| e.prevent_default()),
    );

    // When
    click(&page, [5.0, 5.0]);

    // Then
    assert_eq!(taken(&page), None);
}

#[test]
fn enter_on_a_focused_link_goes_to_its_href_and_space_does_not() {
    // Given: the link focused by Tab
    let page = page();
    key(&page, "Tab");
    assert_eq!(page.doc.borrow().active_element(), Some(page.link));

    // When
    key(&page, " ");
    let after_space = taken(&page);
    key(&page, "Enter");

    // Then
    assert_eq!(after_space, None);
    assert_eq!(taken(&page).as_deref(), Some("?demo=counter"));
}

#[test]
fn an_a_without_href_is_no_link() {
    // Given
    let page = page();

    // When: clicked, and Tab past the link
    click(&page, [5.0, 25.0]);
    key(&page, "Tab");
    key(&page, "Tab");

    // Then
    assert_eq!(taken(&page), None);
    assert_ne!(page.doc.borrow().active_element(), Some(page.plain));
}

#[test]
fn a_link_is_named_by_its_text_with_the_link_role() {
    // Given
    let page = page();

    // When
    let tree = page.doc.borrow().ax_tree().expect("an updated document");

    // Then: the `a` without href is no node of its own
    let roles: Vec<(String, String)> = tree
        .iter()
        .map(|node| (node.role.clone(), node.name.clone()))
        .collect();
    assert_eq!(roles, [("link".to_owned(), "Counter".to_owned())]);
}
