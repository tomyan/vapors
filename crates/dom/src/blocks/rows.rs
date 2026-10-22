//! One row of a keyed `{for}`: its key, index and item signals, the
//! marker that holds its position, and the fragment built for it; and
//! creating, updating, moving and removing one.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_signals::Signal;

use super::Fragment;
use crate::{Dom, NodeId, Root};

pub(super) struct Row<K, V> {
    pub(super) key: K,
    pub(super) index: Signal<usize>,
    pub(super) item: Signal<V>,
    /// An empty text node placed first, so a row with no nodes still
    /// has a position.
    pub(super) marker: NodeId,
    pub(super) fragment: Fragment,
}

/// A row builder: dom, parent, insert-before, index signal, item signal.
pub trait RowBuilder<D, V>:
    Fn(&Rc<RefCell<D>>, NodeId, Option<NodeId>, Signal<usize>, Signal<V>) -> Vec<Root>
{
}
impl<D, V, F> RowBuilder<D, V> for F where
    F: Fn(&Rc<RefCell<D>>, NodeId, Option<NodeId>, Signal<usize>, Signal<V>) -> Vec<Root>
{
}

/// A row's marker and then its fragment's nodes as of now.
pub(super) fn row_nodes<K, V>(row: &Row<K, V>) -> Vec<NodeId> {
    let mut nodes = vec![row.marker];
    nodes.extend(row.fragment.nodes());
    nodes
}

/// A kept row: its index and item updated, moved before `move_before`
/// when it is not on the run of rows that stay.
pub(super) fn reuse<D: Dom, K, V: PartialEq + Clone + 'static>(
    dom: &Rc<RefCell<D>>,
    parent: NodeId,
    mut row: Row<K, V>,
    (index, item): (usize, &V),
    move_before: Option<NodeId>,
) -> Row<K, V> {
    update(&mut row, index, item);
    if let Some(before) = move_before {
        place_before(dom, parent, &row, before);
    }
    row
}

fn update<K, V: PartialEq + Clone + 'static>(row: &mut Row<K, V>, index: usize, item: &V) {
    if row.index.get_untracked() != index {
        row.index.set(index);
    }
    if row.item.with_untracked(|current| current != item) {
        row.item.set(item.clone());
    }
}

fn place_before<D: Dom, K, V>(
    dom: &Rc<RefCell<D>>,
    parent: NodeId,
    row: &Row<K, V>,
    before: NodeId,
) {
    for node in row_nodes(row) {
        dom.borrow_mut().insert_before(parent, node, Some(before));
    }
}

pub(super) fn create<D: Dom, K, V: Clone + 'static>(
    dom: &Rc<RefCell<D>>,
    parent: NodeId,
    before: NodeId,
    key: K,
    index: usize,
    item: V,
    row: &impl RowBuilder<D, V>,
) -> Row<K, V> {
    let marker = dom.borrow_mut().create_text("");
    dom.borrow_mut().insert_before(parent, marker, Some(before));
    let index = Signal::new(index);
    let item = Signal::new(item);
    let fragment = Fragment::build(dom, parent, Some(before), |dom, parent, before| {
        row(dom, parent, before, index, item)
    });
    Row {
        key,
        index,
        item,
        marker,
        fragment,
    }
}

pub(super) fn remove<D: Dom, K, V: 'static>(dom: &Rc<RefCell<D>>, row: Row<K, V>) {
    row.fragment.remove(dom);
    dom.borrow_mut().remove(row.marker);
    row.index.dispose();
    row.item.dispose();
}
