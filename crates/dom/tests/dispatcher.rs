//! A component's own events (`Dispatcher`): dispatched at a node and up
//! through its ancestors at once, as the DOM's `dispatchEvent`, answering
//! whether no listener cancelled it. Refusals: `dispatcher_refusals.rs`.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::recording::RecordingDom;
use vapors_dom::{Dispatcher, Dom, DynDom, Event};

#[path = "support/dispatch_tree.rs"]
mod dispatch_tree;
use dispatch_tree::tree;

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
fn a_listener_may_dispatch_again_and_the_inner_dispatch_finishes_inside_it() {
    // Given: a list whose `optionchoose` listener, hearing it from the
    // first option, dispatches it again at a second option
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let (list, first, second) = {
        let mut d = dom.borrow_mut();
        let list = d.create_element("div");
        let first = d.create_element("div");
        let second = d.create_element("div");
        d.append(list, first);
        d.append(list, second);
        (list, first, second)
    };
    let events = Dispatcher::new(dom.dyn_dom(), &["optionchoose"]);
    let log = Rc::new(RefCell::new(Vec::new()));
    let (into, again) = (Rc::clone(&log), events.clone());
    dom.borrow_mut().listen(
        list,
        "optionchoose",
        Box::new(move |e: &mut Event| {
            into.borrow_mut()
                .push(format!("list heard {:?}", e.target()));
            if e.target() == Some(first) {
                let inner = again.dispatch(second, Event::new("optionchoose"));
                into.borrow_mut().push(format!("inner {inner:?}"));
            }
        }),
    );

    // When
    let outer = events.dispatch(first, Event::new("optionchoose"));

    // Then: the inner dispatch ran to completion inside the outer listener
    assert_eq!(outer, Ok(true));
    assert_eq!(
        *log.borrow(),
        [
            format!("list heard {:?}", Some(first)),
            format!("list heard {:?}", Some(second)),
            "inner Ok(true)".to_owned(),
        ]
    );
}
