//! The recording fake: it records every call, keeps the tree so order
//! can be checked, and dispatches events to listeners with bubbling.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::recording::{Call, RecordingDom};
use vapors_dom::{Dom, NodeId};

fn dom() -> Rc<RefCell<RecordingDom>> {
    Rc::new(RefCell::new(RecordingDom::new()))
}

#[test]
fn records_calls_in_order_and_keeps_the_tree() {
    // Given
    let dom = dom();
    let root = dom.borrow().root();

    // When
    let div = dom.borrow_mut().create_element("div");
    dom.borrow_mut().append(root, div);
    let text = dom.borrow_mut().create_text("hi");
    dom.borrow_mut().append(div, text);
    dom.borrow_mut().set_attribute(div, "class", "box");
    dom.borrow_mut().set_text(text, "hello");

    // Then
    assert_eq!(
        dom.borrow().calls(),
        [
            Call::CreateElement {
                node: div,
                tag: "div".into()
            },
            Call::Append {
                parent: root,
                child: div
            },
            Call::CreateText {
                node: text,
                text: "hi".into()
            },
            Call::Append {
                parent: div,
                child: text
            },
            Call::SetAttribute {
                node: div,
                name: "class".into(),
                value: "box".into()
            },
            Call::SetText {
                node: text,
                text: "hello".into()
            },
        ]
    );
    assert_eq!(dom.borrow().to_html(), r#"<div class="box">hello</div>"#);
}

#[test]
fn insert_before_and_remove_change_the_order() {
    // Given
    let dom = dom();
    let root = dom.borrow().root();
    let a = dom.borrow_mut().create_text("a");
    let b = dom.borrow_mut().create_text("b");
    let c = dom.borrow_mut().create_text("c");
    dom.borrow_mut().append(root, a);
    dom.borrow_mut().append(root, c);

    // When
    dom.borrow_mut().insert_before(root, b, Some(c));
    assert_eq!(dom.borrow().to_html(), "abc");
    dom.borrow_mut().insert_before(root, a, None);
    assert_eq!(dom.borrow().to_html(), "bca");
    dom.borrow_mut().remove(c);

    // Then
    assert_eq!(dom.borrow().to_html(), "ba");
    assert_eq!(dom.borrow().children(root), [b, a]);
}

#[test]
fn calls_can_be_cleared_to_watch_one_step() {
    // Given
    let dom = dom();
    let root = dom.borrow().root();
    let text = dom.borrow_mut().create_text("x");
    dom.borrow_mut().append(root, text);

    // When
    dom.borrow_mut().clear_calls();
    dom.borrow_mut().set_text(text, "y");

    // Then
    assert_eq!(dom.borrow().calls().len(), 1);
}

#[test]
fn node_ids_are_distinct_and_the_root_is_first() {
    // Given
    let dom = dom();

    // When
    let a = dom.borrow_mut().create_element("a");
    let b = dom.borrow_mut().create_element("b");

    // Then
    assert_ne!(a, b);
    assert_ne!(a, dom.borrow().root());
    assert_eq!(dom.borrow().root(), NodeId(0));
}
