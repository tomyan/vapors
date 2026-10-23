//! The worked todo list: `src/todo_list.wisp` with `{if}`/`{else}`, a
//! keyed `{for}` of `TodoItem` children and `bind:value`, compiled by
//! build.rs and driven by the recording Dom.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::todo_list::{TodoList, TodoListProps};
use vapors_dom::PropertyValue;
use vapors_dom::recording::{Call, RecordingDom};

fn mounted() -> (Rc<RefCell<RecordingDom>>, TodoList) {
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();
    let list = TodoList::mount(&dom, root, TodoListProps::default());
    dom.borrow_mut().clear_calls();
    (dom, list)
}

fn items(entries: &[(u32, &str)]) -> Vec<(u32, String)> {
    entries
        .iter()
        .map(|(key, label)| (*key, (*label).to_string()))
        .collect()
}

#[test]
fn an_empty_list_shows_the_else_branch() {
    // When
    let (dom, _list) = mounted();

    // Then
    assert_eq!(
        dom.borrow().to_html(),
        "<input></input><p>Nothing to do</p>"
    );
}

#[test]
fn items_render_one_child_component_each_in_order() {
    // Given
    let (dom, list) = mounted();

    // When
    list.items.set(items(&[(1, "milk"), (2, "eggs")]));

    // Then
    assert_eq!(
        dom.borrow().to_html(),
        "<input></input><ul><li>milk</li><li>eggs</li></ul>"
    );
}

#[test]
fn reordering_moves_the_existing_rows_and_creates_nothing() {
    // Given
    let (dom, list) = mounted();
    list.items
        .set(items(&[(1, "milk"), (2, "eggs"), (3, "tea")]));
    dom.borrow_mut().clear_calls();

    // When
    list.items
        .set(items(&[(3, "tea"), (1, "milk"), (2, "eggs")]));

    // Then
    assert_eq!(
        dom.borrow().to_html(),
        "<input></input><ul><li>tea</li><li>milk</li><li>eggs</li></ul>"
    );
    let calls = dom.borrow().calls().to_vec();
    assert!(!calls.is_empty(), "a reorder places at least one row");
    assert!(
        calls
            .iter()
            .all(|call| matches!(call, Call::InsertBefore { .. })),
        "only moves: {calls:?}"
    );
}

#[test]
fn typing_into_the_input_sets_the_bound_signal() {
    // Given
    let (dom, list) = mounted();
    let input = dom.borrow().children(dom.borrow().root())[0];

    // When
    RecordingDom::input(&dom, input, "bread");

    // Then
    assert_eq!(list.draft.get(), "bread");
}

#[test]
fn setting_the_bound_signal_sets_the_input_value_property() {
    // Given
    let (dom, list) = mounted();
    let input = dom.borrow().children(dom.borrow().root())[0];

    // When
    list.draft.set("jam".to_string());

    // Then
    assert_eq!(
        dom.borrow().property(input, "value"),
        Some(PropertyValue::Text("jam".to_string()))
    );
}
