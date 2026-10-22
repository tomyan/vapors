//! The keyed `{for}` block runtime, per row: insertion and removal
//! touch only those rows, a changed item updates its row in place, and
//! a removed row's effects are disposed.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::blocks::keyed;
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

/// A row showing `index: item`, patched by its own effect.
fn row(
    dom: &Shared,
    parent: NodeId,
    before: Option<NodeId>,
    index: Signal<usize>,
    item: Signal<String>,
) -> Vec<Root> {
    let li = dom.borrow_mut().create_element("li");
    dom.borrow_mut().insert_before(parent, li, before);
    let text = dom.borrow_mut().create_text("");
    dom.borrow_mut().append(li, text);
    let dom = Rc::clone(dom);
    Effect::new(move || {
        let value = format!("{}: {}", index.get(), item.get());
        dom.borrow_mut().set_text(text, &value);
    });
    vec![Root::Node(li)]
}

fn mount(dom: &Shared, root: NodeId, anchor: NodeId, items: Signal<Vec<String>>) {
    keyed(
        dom,
        root,
        anchor,
        move || {
            items
                .get()
                .into_iter()
                .map(|item| (item.clone(), item))
                .collect()
        },
        row,
    );
}

fn creates(dom: &Shared) -> usize {
    dom.borrow()
        .calls()
        .iter()
        .filter(|call| matches!(call, Call::CreateElement { .. }))
        .count()
}

#[test]
fn inserting_and_removing_touch_only_those_rows() {
    // Given
    let owner = Owner::new();
    owner.set();
    let (dom, root, anchor) = setup();
    let items = Signal::new(vec!["a".to_string(), "c".to_string()]);
    mount(&dom, root, anchor, items);
    dom.borrow_mut().clear_calls();

    // When
    items.set(vec!["a".to_string(), "b".to_string(), "c".to_string()]);

    // Then
    assert_eq!(
        dom.borrow().to_html(),
        "<li>0: a</li><li>1: b</li><li>2: c</li>"
    );
    assert_eq!(creates(&dom), 1);

    // When
    dom.borrow_mut().clear_calls();
    items.set(vec!["b".to_string(), "c".to_string()]);

    // Then
    assert_eq!(dom.borrow().to_html(), "<li>0: b</li><li>1: c</li>");
    assert_eq!(creates(&dom), 0);
    let removes = dom
        .borrow()
        .calls()
        .iter()
        .filter(|call| matches!(call, Call::Remove { .. }))
        .count();
    assert_eq!(
        removes,
        2,
        "the row's element and its marker: {:?}",
        dom.borrow().calls()
    );
}

#[test]
fn a_changed_item_with_the_same_key_updates_its_row_in_place() {
    // Given: keyed by id, the label can change
    let owner = Owner::new();
    owner.set();
    let (dom, root, anchor) = setup();
    let items = Signal::new(vec![(1u32, "one".to_string())]);
    keyed(&dom, root, anchor, move || items.get(), row);
    dom.borrow_mut().clear_calls();

    // When
    items.set(vec![(1, "uno".to_string())]);

    // Then
    assert_eq!(dom.borrow().to_html(), "<li>0: uno</li>");
    assert_eq!(creates(&dom), 0);
}

#[test]
fn removed_rows_have_their_effects_disposed() {
    // Given
    let owner = Owner::new();
    owner.set();
    let (dom, root, anchor) = setup();
    let items = Signal::new(vec!["a".to_string()]);
    let ticks = Signal::new(0);
    let runs = Rc::new(RefCell::new(0));
    keyed(
        &dom,
        root,
        anchor,
        move || {
            items
                .get()
                .into_iter()
                .map(|item| (item.clone(), item))
                .collect()
        },
        {
            let runs = Rc::clone(&runs);
            move |dom: &Shared, parent, before, _index: Signal<usize>, _item: Signal<String>| {
                let runs = Rc::clone(&runs);
                Effect::new(move || {
                    let _ = ticks.get();
                    *runs.borrow_mut() += 1;
                });
                let li = dom.borrow_mut().create_element("li");
                dom.borrow_mut().insert_before(parent, li, before);
                vec![Root::Node(li)]
            }
        },
    );

    // When
    items.set(Vec::new());
    ticks.set(1);

    // Then
    assert_eq!(*runs.borrow(), 1);
}
