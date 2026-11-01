//! `bind:this` (`src/refs.wisp`): an element's node in a signal while it
//! is mounted, inside `{if}` and `{for}` too, and `None` once it is not.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::refs::{Refs, RefsProps};
use vapors_dom::recording::{Call, RecordingDom};
use vapors_signals::Signal;

fn mounted(names: &[&str]) -> (Rc<RefCell<RecordingDom>>, Refs, Signal<Vec<String>>) {
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();
    let names = Signal::new(names.iter().map(|n| (*n).to_owned()).collect());
    let refs = Refs::mount(&dom, root, RefsProps { names });
    (dom, refs, names)
}

/// The elements created with `tag`, in order.
fn created(dom: &Rc<RefCell<RecordingDom>>, tag: &str) -> Vec<vapors_dom::NodeId> {
    dom.borrow()
        .calls()
        .iter()
        .filter_map(|call| match call {
            Call::CreateElement { node, tag: made } if made == tag => Some(*node),
            _ => None,
        })
        .collect()
}

#[test]
fn a_mounted_element_is_in_its_signal() {
    // When
    let (dom, refs, _) = mounted(&["a"]);

    // Then: the p, and the row's i
    assert_eq!(refs.first.get(), Some(created(&dom, "p")[0]));
    assert_eq!(refs.row.get(), Some(created(&dom, "i")[0]));
}

#[test]
fn an_element_its_if_removes_leaves_none() {
    // Given
    let (_dom, refs, _) = mounted(&[]);

    // When
    refs.shown.set(false);

    // Then
    assert_eq!(refs.first.get(), None);
}

#[test]
fn a_removed_row_leaves_none_and_a_row_still_there_keeps_its_node() {
    // Given: one row, then a second; the signal holds the newest
    let (dom, refs, names) = mounted(&["a"]);
    names.set(vec!["a".into(), "b".into()]);
    let b = created(&dom, "i")[1];
    assert_eq!(refs.row.get(), Some(b));

    // When: the first row goes; then the second
    names.set(vec!["b".into()]);
    let after_a = refs.row.get();
    names.set(Vec::new());

    // Then: removing a did not clear b's node; removing b did
    assert_eq!(after_a, Some(b));
    assert_eq!(refs.row.get(), None);
}

#[test]
fn unmounting_the_component_disposes_its_own_signals_without_a_panic() {
    // Given
    let (dom, refs, _) = mounted(&["a"]);
    let first = refs.first;

    // When: its cleanups run after its signals are disposed
    refs.unmount(&dom);

    // Then: the signal is gone, and nothing panicked clearing it
    assert!(first.try_get_untracked().is_err());
}
