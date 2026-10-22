//! The `{if}` block runtime: nodes are built when the condition turns
//! true and removed when it turns false, with the branch's own signals
//! and effects disposed; nothing happens while the condition keeps its
//! value.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::blocks::conditional;
use vapors_dom::recording::{Call, RecordingDom};
use vapors_dom::{Dom, NodeId, Root};
use vapors_signals::{Effect, Owner, Signal};

type Shared = Rc<RefCell<RecordingDom>>;

fn setup() -> (Shared, NodeId, NodeId) {
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();
    let anchor = dom.borrow_mut().create_text("");
    dom.borrow_mut().append(root, anchor);
    (dom, root, anchor)
}

fn paragraph(dom: &Shared, parent: NodeId, before: Option<NodeId>, text: &str) -> Vec<Root> {
    let p = dom.borrow_mut().create_element("p");
    dom.borrow_mut().insert_before(parent, p, before);
    let t = dom.borrow_mut().create_text(text);
    dom.borrow_mut().append(p, t);
    vec![Root::Node(p)]
}

#[test]
fn builds_the_live_branch_before_the_anchor_and_swaps_on_change() {
    // Given
    let owner = Owner::new();
    owner.set();
    let (dom, root, anchor) = setup();
    let shown = Signal::new(true);

    // When
    conditional(
        &dom,
        root,
        anchor,
        move || shown.get(),
        |dom, parent, before| paragraph(dom, parent, before, "yes"),
        |dom, parent, before| paragraph(dom, parent, before, "no"),
    );

    // Then
    assert_eq!(dom.borrow().to_html(), "<p>yes</p>");
    shown.set(false);
    assert_eq!(dom.borrow().to_html(), "<p>no</p>");
    shown.set(true);
    assert_eq!(dom.borrow().to_html(), "<p>yes</p>");
}

#[test]
fn an_unchanged_condition_touches_nothing() {
    // Given
    let owner = Owner::new();
    owner.set();
    let (dom, root, anchor) = setup();
    let count = Signal::new(1);
    conditional(
        &dom,
        root,
        anchor,
        move || count.get() > 0,
        |dom, parent, before| paragraph(dom, parent, before, "positive"),
        |_dom, _parent, _before| Vec::new(),
    );
    dom.borrow_mut().clear_calls();

    // When
    count.set(2);

    // Then
    assert_eq!(dom.borrow().calls(), []);
}

#[test]
fn the_hidden_branch_has_its_effects_disposed() {
    // Given
    let owner = Owner::new();
    owner.set();
    let (dom, root, anchor) = setup();
    let shown = Signal::new(true);
    let value = Signal::new(0);
    let runs = Rc::new(RefCell::new(0));
    conditional(
        &dom,
        root,
        anchor,
        move || shown.get(),
        {
            let runs = Rc::clone(&runs);
            move |dom, parent, before| {
                let runs = Rc::clone(&runs);
                Effect::new(move || {
                    let _ = value.get();
                    *runs.borrow_mut() += 1;
                });
                paragraph(dom, parent, before, "on")
            }
        },
        |_dom, _parent, _before| Vec::new(),
    );
    assert_eq!(*runs.borrow(), 1);

    // When
    shown.set(false);
    value.set(1);

    // Then
    assert_eq!(*runs.borrow(), 1);
    assert_eq!(dom.borrow().to_html(), "");
    assert!(
        dom.borrow()
            .calls()
            .iter()
            .any(|call| matches!(call, Call::Remove { .. }))
    );
}

#[test]
fn a_signal_read_while_building_does_not_rebuild_the_branch() {
    // Given
    let owner = Owner::new();
    owner.set();
    let (dom, root, anchor) = setup();
    let shown = Signal::new(true);
    let label = Signal::new("a");
    let evaluations = Rc::new(RefCell::new(0));
    conditional(
        &dom,
        root,
        anchor,
        {
            let evaluations = Rc::clone(&evaluations);
            move || {
                *evaluations.borrow_mut() += 1;
                shown.get()
            }
        },
        move |dom, parent, before| paragraph(dom, parent, before, label.get()),
        |_dom, _parent, _before| Vec::new(),
    );
    dom.borrow_mut().clear_calls();

    // When
    label.set("b");

    // Then: the block's effect did not re-run, let alone touch the dom
    assert_eq!(*evaluations.borrow(), 1);
    assert_eq!(dom.borrow().calls(), []);
}
