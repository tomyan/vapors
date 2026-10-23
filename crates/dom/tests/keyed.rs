//! The keyed `{for}` block runtime: rows are matched by key across
//! runs; a reorder moves nodes instead of recreating them, a changed
//! item updates its row's signal, a removed row is disposed.

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
        "test.wisp:1:1",
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
fn renders_rows_in_order() {
    // Given
    let owner = Owner::new();
    owner.set();
    let (dom, root, anchor) = setup();
    let items = Signal::new(vec!["a".to_string(), "b".to_string()]);

    // When
    mount(&dom, root, anchor, items);

    // Then
    assert_eq!(dom.borrow().to_html(), "<li>0: a</li><li>1: b</li>");
}

#[test]
fn a_reorder_moves_rows_instead_of_recreating_them() {
    // Given
    let owner = Owner::new();
    owner.set();
    let (dom, root, anchor) = setup();
    let items = Signal::new(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    mount(&dom, root, anchor, items);
    dom.borrow_mut().clear_calls();

    // When
    items.set(vec!["c".to_string(), "a".to_string(), "b".to_string()]);

    // Then
    assert_eq!(
        dom.borrow().to_html(),
        "<li>0: c</li><li>1: a</li><li>2: b</li>"
    );
    assert_eq!(creates(&dom), 0);
    let moves = dom
        .borrow()
        .calls()
        .iter()
        .filter(|call| matches!(call, Call::InsertBefore { .. }))
        .count();
    assert_eq!(
        moves,
        2,
        "one row: its marker and its element: {:?}",
        dom.borrow().calls()
    );
}

#[test]
fn swapping_two_rows_is_one_move() {
    // Given
    let owner = Owner::new();
    owner.set();
    let (dom, root, anchor) = setup();
    let items = Signal::new(vec!["a".to_string(), "b".to_string()]);
    mount(&dom, root, anchor, items);
    dom.borrow_mut().clear_calls();

    // When
    items.set(vec!["b".to_string(), "a".to_string()]);

    // Then
    assert_eq!(dom.borrow().to_html(), "<li>0: b</li><li>1: a</li>");
    let moves = dom
        .borrow()
        .calls()
        .iter()
        .filter(|call| matches!(call, Call::InsertBefore { .. }))
        .count();
    assert_eq!(
        moves,
        2,
        "one row: its marker and its element: {:?}",
        dom.borrow().calls()
    );
}

#[test]
fn an_unchanged_list_touches_nothing() {
    // Given
    let owner = Owner::new();
    owner.set();
    let (dom, root, anchor) = setup();
    let items = Signal::new(vec!["a".to_string(), "b".to_string()]);
    mount(&dom, root, anchor, items);
    dom.borrow_mut().clear_calls();

    // When
    items.set(vec!["a".to_string(), "b".to_string()]);

    // Then
    assert_eq!(dom.borrow().calls(), []);
}
