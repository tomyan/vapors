//! Tab decides by the same focusability `Document::focus` uses, from
//! computed styles brought up to date (review-fable-input M2), as
//! Chromium updates style before sequential focus navigation: an
//! element hidden since the last update is skipped, one shown since is
//! visited, and neither leaves focus stuck.

mod support;

use support::focus_page::{page, tab};
use vapors_dom::Dom;
use vapors_elements::{Document, Modifiers};

#[test]
fn tab_skips_an_element_hidden_since_the_last_update() {
    // Given: save focused; note hidden by an attribute, no update yet
    let page = page();
    Document::focus(&page.doc, Some(page.save));
    page.doc
        .borrow_mut()
        .set_attribute(page.note, "style", "display: none");

    // When: lands on cancel (gone is disabled, aside tabindex -1)
    let used = Document::key_pressed(&page.doc, "Tab", Modifiers::default());

    // Then
    assert!(used);
    assert_eq!(page.doc.borrow().active_element(), Some(page.cancel));
}

#[test]
fn tab_visits_an_element_shown_since_the_last_update() {
    // Given: note focused; the hidden box shown by an attribute, no
    // update yet
    let page = page();
    Document::focus(&page.doc, Some(page.note));
    let hidden = page
        .doc
        .borrow()
        .elements()
        .into_iter()
        .find(|node| page.doc.borrow().attribute(*node, "class") == Some("hidden"))
        .expect("the page has a hidden box");
    page.doc
        .borrow_mut()
        .set_attribute(hidden, "style", "display: block");

    // When
    tab(&page.doc, false);

    // Then: the shown box, before cancel
    assert_eq!(page.doc.borrow().active_element(), Some(hidden));
}

#[test]
fn tab_past_the_last_element_leaves_the_page_when_the_rest_are_hidden_since() {
    // Given: note focused, cancel hidden since the last update
    let page = page();
    Document::focus(&page.doc, Some(page.note));
    page.doc
        .borrow_mut()
        .set_attribute(page.cancel, "style", "display: none");

    // When
    tab(&page.doc, false);

    // Then
    assert_eq!(page.doc.borrow().active_element(), None);
}
