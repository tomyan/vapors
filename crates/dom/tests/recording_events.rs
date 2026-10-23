//! The recording fake's event dispatch: target first, then bubbling,
//! stopped by `stop_propagation`; listeners may use the dom.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::recording::RecordingDom;
use vapors_dom::{Dom, PropertyValue};

fn dom() -> Rc<RefCell<RecordingDom>> {
    Rc::new(RefCell::new(RecordingDom::new()))
}

#[test]
fn click_dispatches_to_the_target_then_bubbles() {
    // Given
    let dom = dom();
    let root = dom.borrow().root();
    let outer = dom.borrow_mut().create_element("div");
    let inner = dom.borrow_mut().create_element("button");
    dom.borrow_mut().append(root, outer);
    dom.borrow_mut().append(outer, inner);
    let log = Rc::new(RefCell::new(Vec::new()));
    for (node, name) in [(inner, "inner"), (outer, "outer")] {
        let log = Rc::clone(&log);
        dom.borrow_mut().listen(
            node,
            "click",
            Box::new(move |_event| log.borrow_mut().push(name)),
        );
    }

    // When
    RecordingDom::click(&dom, inner);

    // Then
    assert_eq!(*log.borrow(), ["inner", "outer"]);
}

#[test]
fn stop_propagation_keeps_the_event_from_outer_listeners() {
    // Given
    let dom = dom();
    let root = dom.borrow().root();
    let outer = dom.borrow_mut().create_element("div");
    let inner = dom.borrow_mut().create_element("button");
    dom.borrow_mut().append(root, outer);
    dom.borrow_mut().append(outer, inner);
    let log = Rc::new(RefCell::new(Vec::new()));
    {
        let log = Rc::clone(&log);
        dom.borrow_mut().listen(
            inner,
            "click",
            Box::new(move |event| {
                event.stop_propagation();
                log.borrow_mut().push("inner");
            }),
        );
    }
    {
        let log = Rc::clone(&log);
        dom.borrow_mut().listen(
            outer,
            "click",
            Box::new(move |_event| log.borrow_mut().push("outer")),
        );
    }

    // When
    RecordingDom::click(&dom, inner);

    // Then
    assert_eq!(*log.borrow(), ["inner"]);
}

#[test]
fn a_listener_may_use_the_dom_while_it_runs() {
    // Given: the listener borrows the dom, so dispatch must not hold it
    let dom = dom();
    let root = dom.borrow().root();
    let button = dom.borrow_mut().create_element("button");
    dom.borrow_mut().append(root, button);
    let text = dom.borrow_mut().create_text("0");
    dom.borrow_mut().append(button, text);
    {
        let dom = Rc::clone(&dom);
        dom.clone().borrow_mut().listen(
            button,
            "click",
            Box::new(move |_event| dom.borrow_mut().set_text(text, "1")),
        );
    }

    // When
    RecordingDom::click(&dom, button);

    // Then
    assert_eq!(dom.borrow().to_html(), "<button>1</button>");
}

#[test]
fn input_carries_the_value_and_change_carries_checked() {
    // Given
    let dom = dom();
    let root = dom.borrow().root();
    let input = dom.borrow_mut().create_element("input");
    dom.borrow_mut().append(root, input);
    let seen = Rc::new(RefCell::new((String::new(), false)));
    {
        let seen = Rc::clone(&seen);
        dom.borrow_mut().listen(
            input,
            "input",
            Box::new(move |event| seen.borrow_mut().0 = event.value().to_string()),
        );
    }
    {
        let seen = Rc::clone(&seen);
        dom.borrow_mut().listen(
            input,
            "change",
            Box::new(move |event| seen.borrow_mut().1 = event.checked()),
        );
    }

    // When
    RecordingDom::input(&dom, input, "typed");
    RecordingDom::change(&dom, input, true);

    // Then
    assert_eq!(*seen.borrow(), ("typed".to_string(), true));
    assert_eq!(
        dom.borrow().property(input, "value"),
        Some(PropertyValue::Text("typed".into()))
    );
}
