//! A focused text field whose `html` then goes `display: none`: the
//! update, the caret and selection the platform reads, typing, an IME
//! composition, selection keys and Tab all run without panicking over
//! the empty box tree. The field keeps focus and takes the typing:
//! vapors has no focus fixup yet (Chromium blurs a focused element that
//! loses its box at the next update), for html or any other element.

mod support;

use support::text_page::{Page, click, key, page, shift, type_text};
use support::viewport;
use vapors_dom::Dom;
use vapors_elements::{Document, TextEdit};

/// The page with "Warm" focused, then html hidden and updated.
fn focused_then_hidden() -> Page {
    let page = page();
    click(&page, [5.0, 12.0]);
    update(&page);
    let root = page.doc.borrow().root();
    page.doc
        .borrow_mut()
        .set_attribute(root, "style", "display: none");
    update(&page);
    page
}

fn update(page: &Page) {
    page.doc
        .borrow_mut()
        .update(viewport(400.0, 100.0))
        .expect("updates");
}

#[test]
fn the_platform_reads_no_text_input_when_html_is_hidden() {
    // Given
    let page = focused_then_hidden();

    // When
    let input = page.doc.borrow_mut().text_input();

    // Then
    assert_eq!(input, None);
}

#[test]
fn typing_selecting_composing_and_tab_run_when_html_is_hidden_and_tab_focuses_nothing() {
    // Given
    let page = focused_then_hidden();

    // When
    type_text(&page, "er");
    let (left, modifiers) = shift("ArrowLeft");
    Document::key_pressed(&page.doc, left, modifiers);
    let edit = TextEdit {
        before: "erWarm".to_owned(),
        text: "eにWarm".to_owned(),
        selection: (2, 2),
        composing: Some((1, 2)),
    };
    let edited = Document::text_edited(&page.doc, &edit);
    key(&page, "Tab");
    update(&page);

    // Then: the edit applied, and Tab found nothing rendered to focus
    assert_eq!(edited, Ok(()));
    assert_eq!(page.doc.borrow().active_element(), None);
}
