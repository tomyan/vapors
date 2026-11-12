//! The focus fixup's tests: a text field typed in, then the update and
//! the settle the app runs after it.

use std::rc::Rc;

use vapors_dom::{Dom, Event};
use vapors_elements::Document;

use super::text_page::{Page, click, page, type_text};
use super::viewport;

/// "Warm" focused by a click, "er" typed at its start, `blur` and
/// `focusout` logged with `input` and `change`, the log then cleared.
pub fn typed_in() -> Page {
    let page = page();
    for kind in ["blur", "focusout"] {
        let into = Rc::clone(&page.log);
        let handler = move |e: &mut Event| into.borrow_mut().push(e.name().to_owned());
        page.doc
            .borrow_mut()
            .listen(page.name, kind, Box::new(handler));
    }
    click(&page, [1.0, 12.0]);
    type_text(&page, "er");
    page.log.borrow_mut().clear();
    page
}

/// The update after a change, then the settle the app runs after it.
pub fn update_and_settle(page: &Page) {
    page.doc
        .borrow_mut()
        .update(viewport(400.0, 100.0))
        .expect("updates");
    Document::settle_focus(&page.doc);
}

/// Hides `page`'s field with `display: none`.
pub fn hide_field(page: &Page) {
    let mut doc = page.doc.borrow_mut();
    doc.set_attribute(page.name, "style", "display: none");
}
