//! A focused text field whose `html` then goes `display: none`: the
//! update, the caret and selection the platform reads, typing, an IME
//! composition, selection keys and Tab all run without panicking over
//! the empty box tree. These run between the update and the settle
//! after it, while the field still has focus; the first key blurs it
//! (the fixup runs before a key, review-fable-input M5), so the rest
//! find nothing focused, as the settle would have left it
//! (`Document::settle_focus`, `tests/focus_fixup.rs`).
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
    let blurred = page.doc.borrow().active_element();
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

    // Then: the first key blurred the hidden field, so the edit found
    // no field, and Tab found nothing rendered to focus
    assert_eq!(blurred, None);
    assert_eq!(
        edited,
        Err("the text input was edited with no text field focused".to_owned())
    );
    assert_eq!(page.doc.borrow().active_element(), None);
}
