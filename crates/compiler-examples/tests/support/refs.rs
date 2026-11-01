//! Mounting `src/refs.wisp` on the recording Dom, and finding what it
//! created.

#![allow(dead_code)] // each test binary uses a subset

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::refs::{Refs, RefsProps};
use vapors_dom::NodeId;
use vapors_dom::recording::{Call, RecordingDom};
use vapors_signals::Signal;

pub type Tree = Rc<RefCell<RecordingDom>>;

pub fn new_dom() -> Tree {
    Rc::new(RefCell::new(RecordingDom::new()))
}

/// Refs with `names` as its rows, and the names signal.
pub fn mounted(names: &[&str]) -> (Tree, Refs, Signal<Vec<String>>) {
    let dom = new_dom();
    let (refs, names) = mounted_with_top(&dom, names, Signal::new(None));
    (dom, refs, names)
}

/// Refs in `dom`, its div bound into `top`, and the names signal.
pub fn mounted_with_top(
    dom: &Tree,
    names: &[&str],
    top: Signal<Option<NodeId>>,
) -> (Refs, Signal<Vec<String>>) {
    let root = dom.borrow().root();
    let names = Signal::new(names.iter().map(|n| (*n).to_owned()).collect());
    (Refs::mount(dom, root, RefsProps { names, top }), names)
}

/// The elements created with `tag`, in order.
pub fn created(dom: &Tree, tag: &str) -> Vec<NodeId> {
    dom.borrow()
        .calls()
        .iter()
        .filter_map(|call| match call {
            Call::CreateElement { node, tag: made } if made == tag => Some(*node),
            _ => None,
        })
        .collect()
}

/// The row element holding the text `name`, among those created.
pub fn row(dom: &Tree, name: &str) -> NodeId {
    let tree = dom.borrow();
    let text = tree.calls().iter().find_map(|call| match call {
        Call::CreateText { node, text } | Call::SetText { node, text } if text == name => {
            Some(*node)
        }
        _ => None,
    });
    let rows = created(dom, "i");
    *rows
        .iter()
        .find(|node| text.is_some_and(|text| tree.children(**node).contains(&text)))
        .unwrap_or_else(|| panic!("no row for {name:?} among {rows:?}"))
}
