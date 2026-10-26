//! Mounting a ported Svelte sample on the recording Dom, and finding
//! its elements by tag, for the `svelte_snippets*` tests.

#![allow(dead_code)] // each test binary uses a subset

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::NodeId;
use vapors_dom::recording::{Call, RecordingDom};

pub type Shared = Rc<RefCell<RecordingDom>>;

/// A recording Dom with `mount` run on its root; the component is kept
/// with it so its effects live.
pub fn mounted<C>(mount: impl FnOnce(&Shared, NodeId) -> C) -> (Shared, C) {
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();
    let component = mount(&dom, root);
    (dom, component)
}

/// The `nth` element created with `tag`, in creation order.
pub fn element(dom: &Shared, tag: &str, nth: usize) -> NodeId {
    let created: Vec<NodeId> = dom
        .borrow()
        .calls()
        .iter()
        .filter_map(|call| match call {
            Call::CreateElement { node, tag: made } if made == tag => Some(*node),
            _ => None,
        })
        .collect();
    *created
        .get(nth)
        .unwrap_or_else(|| panic!("no <{tag}> number {nth}; created {created:?}"))
}

pub fn html(dom: &Shared) -> String {
    dom.borrow().to_html()
}
