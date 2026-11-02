//! While the window is away (the lead, on review-v100 of #112): a focus
//! move fires nothing until the return, when the element focused then
//! gets `focus` and `focusin` once, as Chromium fires them only when the
//! page regains focus; and the loss cancels what the pointer holds, so a
//! range's drag ends with `pointercancel` and no `change`, and a move
//! after the return drags nothing.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, Event, NodeId};
use vapors_elements::Document;

/// Each of `names` on `node`, logged as `label name`.
fn log_on(
    doc: &Rc<RefCell<Document>>,
    log: &Rc<RefCell<Vec<String>>>,
    node: NodeId,
    label: &'static str,
    names: &[&'static str],
) {
    for name in names {
        let log = Rc::clone(log);
        doc.borrow_mut().listen(
            node,
            name,
            Box::new(move |e: &mut Event| log.borrow_mut().push(format!("{label} {}", e.name()))),
        );
    }
}

#[test]
fn a_focus_move_while_the_window_is_away_fires_once_on_the_return() {
    // Given: amount focused, then the window lost
    let page = support::number_page::page();
    Document::focus(&page.doc, Some(page.amount));
    let log = Rc::new(RefCell::new(Vec::new()));
    let names = ["blur", "focusout", "focus", "focusin"];
    log_on(&page.doc, &log, page.amount, "amount", &names);
    log_on(&page.doc, &log, page.scrub, "scrub", &names);
    Document::window_focus(&page.doc, false);
    log.borrow_mut().clear();

    // When: focus moves to scrub while away, then the window returns
    Document::focus(&page.doc, Some(page.scrub));
    let while_away = log.borrow().clone();
    Document::window_focus(&page.doc, true);

    // Then
    assert!(while_away.is_empty(), "{while_away:?}");
    assert_eq!(page.doc.borrow().active_element(), Some(page.scrub));
    assert_eq!(*log.borrow(), ["scrub focus", "scrub focusin"]);
}

#[test]
fn the_window_lost_mid_drag_cancels_it_with_no_change_and_a_later_move_drags_nothing() {
    // Given: a press at 50 on the range, its drag under way
    let page = support::range_page::page();
    let log = Rc::clone(&page.log);
    log_on(&page.doc, &log, page.level, "level", &["pointercancel"]);
    Document::pointer_pressed(&page.doc, [100.0, 10.0]);

    // When: the window lost, then back, then a move and a release
    Document::window_focus(&page.doc, false);
    Document::window_focus(&page.doc, true);
    Document::pointer_moved(&page.doc, [150.0, 10.0]);
    Document::pointer_released(&page.doc, [150.0, 10.0]);

    // Then
    assert_eq!(*page.log.borrow(), ["input 50 50", "level pointercancel"]);
    assert_eq!(support::range_page::value(&page, page.level), "50");
}
