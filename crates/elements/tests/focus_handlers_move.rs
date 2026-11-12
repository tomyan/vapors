//! A `blur` or `focus` handler that moves focus on (review-fable-input
//! M1): the destination is held aside while the element losing focus
//! commits and blurs, so during its `blur` nothing has focus, as
//! Chromium nulls its focused element before the blur; what the handler
//! focused is kept, and the destination hears nothing. The blur case
//! is Chromium 155's log; the focus case and the destination hidden by
//! a blur handler are from reading Blink's `Document::SetFocusedElement`
//! (not probed in the corpus).

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::focus_page::{Page, page};
use vapors_dom::{Dom, Event, NodeId};
use vapors_elements::{Document, Focus};

type Log = Rc<RefCell<Vec<String>>>;

fn log_on(page: &Page, node: NodeId, label: &'static str, log: &Log) {
    for kind in ["focus", "focusin", "blur", "focusout"] {
        let into = Rc::clone(log);
        page.doc.borrow_mut().listen(
            node,
            kind,
            Box::new(move |e: &mut Event| into.borrow_mut().push(format!("{label} {}", e.name()))),
        );
    }
}

/// `event` on `node` focuses `to`.
fn focus_on(page: &Page, node: NodeId, event: &str, to: NodeId) {
    let d = Rc::clone(&page.doc);
    page.doc.borrow_mut().listen(
        node,
        event,
        Box::new(move |_: &mut Event| {
            Document::focus(&d, Some(to));
        }),
    );
}

/// The page with save focused and save, note and cancel logged.
fn save_focused() -> (Page, Log) {
    let page = page();
    let log = Log::default();
    Document::focus(&page.doc, Some(page.save));
    log_on(&page, page.save, "save", &log);
    log_on(&page, page.note, "note", &log);
    log_on(&page, page.cancel, "cancel", &log);
    (page, log)
}

#[test]
fn a_blur_handler_that_moves_focus_on_leaves_the_destination_nothing() {
    // Given: save's blur focuses cancel; focus is moving to note
    let (page, log) = save_focused();
    focus_on(&page, page.save, "blur", page.cancel);

    // When
    let went = Document::focus(&page.doc, Some(page.note));

    // Then: Chromium's log; note hears nothing
    assert_eq!(went, Focus::Done);
    assert_eq!(page.doc.borrow().active_element(), Some(page.cancel));
    assert_eq!(
        *log.borrow(),
        [
            "save blur",
            "cancel focus",
            "cancel focusin",
            "save focusout"
        ]
    );
}

#[test]
fn during_blur_nothing_has_focus() {
    // Given
    let (page, _log) = save_focused();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let (into, d) = (Rc::clone(&seen), Rc::clone(&page.doc));
    page.doc.borrow_mut().listen(
        page.save,
        "blur",
        Box::new(move |_: &mut Event| into.borrow_mut().push(d.borrow().active_element())),
    );

    // When
    Document::focus(&page.doc, Some(page.note));

    // Then: the body, not note
    assert_eq!(*seen.borrow(), [None]);
    assert_eq!(page.doc.borrow().active_element(), Some(page.note));
}

#[test]
fn a_focus_handler_that_moves_focus_on_sends_no_focusin_to_the_first() {
    // Given: note's focus moves on to cancel
    let (page, log) = save_focused();
    focus_on(&page, page.note, "focus", page.cancel);

    // When
    Document::focus(&page.doc, Some(page.note));

    // Then: note has focus while its `focus` fires, so moving on blurs
    // it, and it gets no `focusin`
    assert_eq!(page.doc.borrow().active_element(), Some(page.cancel));
    assert_eq!(
        *log.borrow(),
        [
            "save blur",
            "save focusout",
            "note focus",
            "note blur",
            "note focusout",
            "cancel focus",
            "cancel focusin"
        ]
    );
}

#[test]
fn a_blur_handler_that_hides_the_destination_leaves_focus_on_nothing() {
    // Given: save's blur hides note, where focus was going
    let (page, log) = save_focused();
    let (d, note) = (Rc::clone(&page.doc), page.note);
    page.doc.borrow_mut().listen(
        page.save,
        "blur",
        Box::new(move |_: &mut Event| d.borrow_mut().set_attribute(note, "style", "display: none")),
    );

    // When
    Document::focus(&page.doc, Some(page.note));

    // Then: note is not focusable by the time it would take focus
    assert_eq!(page.doc.borrow().active_element(), None);
    assert_eq!(*log.borrow(), ["save blur", "save focusout"]);
}
