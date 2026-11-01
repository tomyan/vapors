//! A component's own events (`Dispatcher`): dispatched at a node and up
//! through its ancestors at once, as the DOM's `dispatchEvent`, answering
//! whether no listener cancelled it; refused, not dropped, when it cannot
//! run.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::recording::RecordingDom;
use vapors_dom::{Dispatcher, Dom, DynDom, Event};

/// The tree, the list, the option in it, and the log of who heard.
type Tree = (
    Rc<RefCell<RecordingDom>>,
    vapors_dom::NodeId,
    vapors_dom::NodeId,
    Rc<RefCell<Vec<String>>>,
);

/// A list holding one option, and a log of who heard what.
fn tree() -> Tree {
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let log = Rc::new(RefCell::new(Vec::new()));
    let (list, option) = {
        let mut d = dom.borrow_mut();
        let list = d.create_element("div");
        let option = d.create_element("div");
        d.append(list, option);
        (list, option)
    };
    for (node, who) in [(option, "option"), (list, "list")] {
        let into = Rc::clone(&log);
        dom.borrow_mut().listen(
            node,
            "optionchoose",
            Box::new(move |_: &mut Event| into.borrow_mut().push(who.to_owned())),
        );
    }
    (dom, list, option, log)
}

#[test]
fn an_event_reaches_its_node_then_its_ancestors_and_answers_not_cancelled() {
    // Given
    let (dom, _, option, log) = tree();
    let events = Dispatcher::new(dom.dyn_dom(), &["optionchoose"]);

    // When
    let went = events.dispatch(option, Event::new("optionchoose"));

    // Then
    assert_eq!(went, Ok(true));
    assert_eq!(*log.borrow(), ["option", "list"]);
}

#[test]
fn a_listener_that_prevents_the_default_cancels_it() {
    // Given: the list cancels
    let (dom, list, option, _) = tree();
    dom.borrow_mut().listen(
        list,
        "optionchoose",
        Box::new(|e: &mut Event| e.prevent_default()),
    );
    let events = Dispatcher::new(dom.dyn_dom(), &["optionchoose"]);

    // When
    let went = events.dispatch(option, Event::new("optionchoose"));

    // Then
    assert_eq!(went, Ok(false));
}

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
fn a_listener_may_use_the_dom_during_a_dispatch() {
    // Given: the option's listener reads the tree
    let (dom, _, option, _) = tree();
    let reader = Rc::clone(&dom);
    dom.borrow_mut().listen(
        option,
        "optionchoose",
        Box::new(move |_: &mut Event| {
            reader.borrow_mut().create_text("read");
        }),
    );
    let events = Dispatcher::new(dom.dyn_dom(), &["optionchoose"]);

    // Then
    assert_eq!(
        events.dispatch(option, Event::new("optionchoose")),
        Ok(true)
    );
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
#[should_panic(expected = "declares only")]
fn emit_panics_on_a_refusal_naming_it() {
    // Given
    let (dom, _, option, _) = tree();
    let events = Dispatcher::new(dom.dyn_dom(), &["optionchoose"]);

    // When: a name not declared, which only a bug in the component sends
    let _ = events.emit(option, Event::new("optionchose"));
}

#[test]
fn emit_answers_as_dispatch_when_it_runs() {
    // Given
    let (dom, _, option, _) = tree();
    let events = Dispatcher::new(dom.dyn_dom(), &["optionchoose"]);

    // Then
    assert!(events.emit(option, Event::new("optionchoose")));
}
