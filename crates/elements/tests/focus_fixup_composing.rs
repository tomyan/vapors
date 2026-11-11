//! The focus fixup with an IME composing in the field. Chromium
//! 155.0.8059.79 (`node scripts/chromium-focus-fixup.mjs`, "composing")
//! commits the composition: `compositionend`, then `change` with the
//! composed text in the value, `blur`, `focusout`. Here the composed
//! text is already in the value, so `change` carries it, and the
//! composition ends with no event: vapors fires no composition events.

mod support;

use support::fixup::{hide_field, typed_in, update_and_settle};
use support::text_page::{Page, value};
use support::viewport;
use vapors_dom::Dom;
use vapors_elements::{Document, TextEdit};
use vapors_scene::Item;

/// The 1 px lines under composing text in the last update.
fn underlines(page: &Page) -> usize {
    let doc = page.doc.borrow();
    let painted = doc.painted().expect("painted");
    let lines = painted.scene().items().iter().filter(|item| match item {
        Item::Quad(q) => {
            (q.rect.bottom - q.rect.top - 1.0).abs() < 0.01 && q.rect.right - q.rect.left > 4.0
        }
        _ => false,
    });
    lines.count()
}

/// The page typed in, an IME composing "ka" after "er".
fn composing() -> Page {
    let page = typed_in();
    let edit = TextEdit {
        before: "erWarm".into(),
        text: "erkaWarm".into(),
        selection: (4, 4),
        composing: Some((2, 4)),
    };
    Document::text_edited(&page.doc, &edit).expect("applies");
    update_and_settle(&page);
    page.log.borrow_mut().clear();
    page
}

#[test]
fn the_fixup_commits_the_composed_text() {
    // Given
    let page = composing();

    // When
    hide_field(&page);
    update_and_settle(&page);

    // Then
    let log = page.log.borrow().clone();
    assert_eq!(log, ["change \"erkaWarm\"", "blur", "focusout"]);
    assert_eq!(value(&page, page.name), "erkaWarm");
}

#[test]
fn the_composition_ends_at_the_fixup() {
    // Given: underlined while composing
    let page = composing();
    let before = underlines(&page);
    hide_field(&page);
    update_and_settle(&page);

    // When: shown and focused again by script, which moves no caret
    page.doc.borrow_mut().set_attribute(page.name, "style", "");
    update_and_settle(&page);
    Document::focus(&page.doc, Some(page.name));
    page.doc
        .borrow_mut()
        .update(viewport(400.0, 100.0))
        .expect("updates");

    // Then
    assert_eq!(before, 1, "the composition was underlined");
    assert_eq!(underlines(&page), 0);
}
