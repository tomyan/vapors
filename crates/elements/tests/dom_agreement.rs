//! The two `Dom` implementations give one answer: the worked todo list,
//! driven through the same item changes on the recording fake and on a
//! Document, builds the same tree after every change. The blocks
//! runtime's inserts, moves and removals are what is exercised.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::todo_list::{TodoList, TodoListProps};
use vapors_dom::recording::RecordingDom;
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;
use vapors_text::FontSystem;

/// Each step's item list: add, reorder, remove, empty, refill.
fn steps() -> Vec<Vec<(u32, String)>> {
    let list =
        |entries: &[(u32, &str)]| entries.iter().map(|(k, v)| (*k, (*v).to_owned())).collect();
    vec![
        list(&[(1, "milk"), (2, "eggs"), (3, "tea")]),
        list(&[(3, "tea"), (1, "milk"), (2, "eggs")]),
        list(&[(3, "tea"), (2, "bread")]),
        list(&[]),
        list(&[(4, "jam")]),
    ]
}

/// The tree's HTML after mounting and after each step.
fn drive<D: Dom + 'static>(
    dom: &Rc<RefCell<D>>,
    parent: NodeId,
    html: impl Fn(&D) -> String,
) -> Vec<String> {
    let list = TodoList::mount(dom, parent, TodoListProps::default());
    let mut seen = vec![html(&dom.borrow())];
    for items in steps() {
        list.items.set(items);
        seen.push(html(&dom.borrow()));
    }
    seen
}

#[test]
fn a_document_builds_the_tree_the_recording_dom_builds() {
    // Given
    let recording = Rc::new(RefCell::new(RecordingDom::new()));
    let document = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    let (root, body) = (recording.borrow().root(), document.borrow().body());

    // When
    let expected = drive(&recording, root, RecordingDom::to_html);
    let actual = drive(&document, body, |doc: &Document| doc.inner_html(doc.body()));

    // Then
    assert_eq!(actual, expected);
}
