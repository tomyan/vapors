//! A component's own events (`Dispatcher`) refused, not dropped, when
//! they cannot run: before any listener, or after all of them with the
//! cancel answer kept.

use vapors_dom::{Dispatcher, Dom, DynDom, Event};

#[path = "support/dispatch_tree.rs"]
mod dispatch_tree;
use dispatch_tree::tree;

#[test]
fn a_dispatch_while_the_dom_is_borrowed_is_refused() {
    // Given
    let (dom, _, option, log) = tree();
    let events = Dispatcher::new(dom.dyn_dom(), &["optionchoose"]);

    // When: held mutably, then shared
    let held = dom.borrow_mut();
    let while_mut = events.dispatch(option, Event::new("optionchoose"));
    drop(held);
    let shared = dom.borrow();
    let while_shared = events.dispatch(option, Event::new("optionchoose"));
    drop(shared);

    // Then: refused both times, and nobody heard it
    for refused in [while_mut, while_shared] {
        assert!(
            refused
                .expect_err("refused")
                .to_string()
                .contains("borrowed")
        );
    }
    assert!(log.borrow().is_empty());
}

#[test]
fn an_event_the_dispatcher_did_not_declare_is_refused() {
    // Given
    let (dom, _, option, log) = tree();
    let events = Dispatcher::new(dom.dyn_dom(), &["optionchoose"]);

    // When
    let refused = events.dispatch(option, Event::new("optionchose"));

    // Then
    assert!(
        refused
            .expect_err("refused")
            .to_string()
            .contains("declares only")
    );
    assert!(log.borrow().is_empty());
}

#[test]
fn a_dispatch_at_a_removed_node_is_refused_naming_it() {
    // Given
    let (dom, list, option, log) = tree();
    let events = Dispatcher::new(dom.dyn_dom(), &["optionchoose"]);
    dom.borrow_mut().remove(list);

    // When: the option went with its list
    let refused = events.dispatch(option, Event::new("optionchoose"));

    // Then
    let why = refused.expect_err("refused").to_string();
    assert!(
        why.contains("not in the tree") && why.contains(&format!("{option:?}")),
        "{why}"
    );
    assert!(log.borrow().is_empty());
}

#[test]
fn a_capture_asked_of_a_dispatched_event_is_refused_after_every_listener_ran() {
    // Given: the option releases capture (no pressed pointer), the list cancels
    let (dom, list, option, log) = tree();
    dom.borrow_mut().listen(
        option,
        "optionchoose",
        Box::new(|e: &mut Event| e.release_pointer_capture()),
    );
    dom.borrow_mut().listen(
        list,
        "optionchoose",
        Box::new(|e: &mut Event| e.prevent_default()),
    );
    let events = Dispatcher::new(dom.dyn_dom(), &["optionchoose"]);

    // When
    let refused = events
        .dispatch(option, Event::new("optionchoose"))
        .expect_err("refused");

    // Then: every listener ran, and the cancel answer is kept
    assert_eq!(*log.borrow(), ["option", "list"]);
    assert!(
        refused
            .to_string()
            .contains("dispatched to 4 listeners, then refused"),
        "{refused}"
    );
    assert_eq!(refused.cancelled(), Some(true));
}
