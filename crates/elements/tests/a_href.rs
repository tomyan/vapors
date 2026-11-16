//! Links (`<a href>`): a click on one, or on what it holds, asks to go
//! to its `href` unless a listener prevented the click; Enter on a
//! focused link does too, Space does not (as in Chromium); an `a`
//! without `href` is no link. The app takes the address
//! (`Document::take_navigation`) and the platform goes there.

use crate::support;

use support::link_page::{click, key, page, taken};
use vapors_dom::{Dom, Event};

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
