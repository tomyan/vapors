//! `{for}` with keys: rows are matched by key across runs. A kept row
//! has its index and item signals updated; rows not on the longest
//! increasing subsequence of old positions are moved; new rows are
//! built and vanished rows removed.

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

use vapors_signals::{Effect, Owner, Signal, untrack};

use super::Fragment;
use super::sequence::longest_increasing;
use crate::{Dom, NodeId};

struct Row<K, V> {
    key: K,
    index: Signal<usize>,
    item: Signal<V>,
    /// An empty text node placed first, so a row with no nodes still
    /// has a position.
    marker: NodeId,
    fragment: Fragment,
}

/// A row builder: dom, parent, insert-before, index signal, item signal.
pub trait RowBuilder<D, V>:
    Fn(&Rc<RefCell<D>>, NodeId, Option<NodeId>, Signal<usize>, Signal<V>) -> Vec<NodeId>
{
}
impl<D, V, F> RowBuilder<D, V> for F where
    F: Fn(&Rc<RefCell<D>>, NodeId, Option<NodeId>, Signal<usize>, Signal<V>) -> Vec<NodeId>
{
}

/// Keeps one row per `(key, item)` of `items` built before `anchor`.
///
/// # Panics
/// When two items share a key; keys must be unique per row.
pub fn keyed<D, K, V, I, R>(dom: &Rc<RefCell<D>>, parent: NodeId, anchor: NodeId, items: I, row: R)
where
    D: Dom + 'static,
    K: Eq + Hash + Clone + 'static,
    V: PartialEq + Clone + 'static,
    I: Fn() -> Vec<(K, V)> + 'static,
    R: RowBuilder<D, V> + 'static,
{
    let dom = Rc::clone(dom);
    let block_owner = Owner::new();
    let mut rows: Vec<Row<K, V>> = Vec::new();
    Effect::new(move || {
        let next = items();
        let old = std::mem::take(&mut rows);
        rows = untrack(|| block_owner.with(|| reconcile(&dom, parent, anchor, old, next, &row)));
    });
}

fn reconcile<D, K, V>(
    dom: &Rc<RefCell<D>>,
    parent: NodeId,
    anchor: NodeId,
    old: Vec<Row<K, V>>,
    next: Vec<(K, V)>,
    row: &impl RowBuilder<D, V>,
) -> Vec<Row<K, V>>
where
    D: Dom,
    K: Eq + Hash + Clone + 'static,
    V: PartialEq + Clone + 'static,
{
    refuse_duplicate_keys(&next);
    let mut kept: HashMap<K, (usize, Row<K, V>)> = old
        .into_iter()
        .enumerate()
        .map(|(position, row)| (row.key.clone(), (position, row)))
        .collect();
    let old_positions: Vec<Option<usize>> = next
        .iter()
        .map(|(key, _)| kept.get(key).map(|(position, _)| *position))
        .collect();
    let stays = stays(&old_positions);
    let mut built = Vec::with_capacity(next.len());
    let mut before = anchor;
    for (index, (key, item)) in next.into_iter().enumerate().rev() {
        let row = match kept.remove(&key) {
            Some((_, mut existing)) => {
                update(&mut existing, index, &item);
                if !stays[index] {
                    move_before(dom, parent, &existing, before);
                }
                existing
            }
            None => create(dom, parent, before, key, index, item, row),
        };
        before = row.marker;
        built.push(row);
    }
    for (_, (_, row)) in kept {
        remove(dom, row);
    }
    built.reverse();
    built
}

fn refuse_duplicate_keys<K: Eq + Hash, V>(next: &[(K, V)]) {
    let mut seen = std::collections::HashSet::with_capacity(next.len());
    let unique = next.iter().all(|(key, _)| seen.insert(key));
    assert!(
        unique,
        "keyed {{for}}: two rows share a key; give `key=` an expression that is unique per row"
    );
}

/// Which new positions hold a kept row that need not move: the longest
/// increasing subsequence of old positions.
fn stays(old_positions: &[Option<usize>]) -> Vec<bool> {
    let kept: Vec<(usize, usize)> = old_positions
        .iter()
        .enumerate()
        .filter_map(|(new, old)| old.map(|old| (new, old)))
        .collect();
    let positions: Vec<usize> = kept.iter().map(|(_, old)| *old).collect();
    let mut stays = vec![false; old_positions.len()];
    for i in longest_increasing(&positions) {
        stays[kept[i].0] = true;
    }
    stays
}

fn update<K, V: PartialEq + Clone + 'static>(row: &mut Row<K, V>, index: usize, item: &V) {
    if row.index.get_untracked() != index {
        row.index.set(index);
    }
    if row.item.with_untracked(|current| current != item) {
        row.item.set(item.clone());
    }
}

fn move_before<D: Dom, K, V>(
    dom: &Rc<RefCell<D>>,
    parent: NodeId,
    row: &Row<K, V>,
    before: NodeId,
) {
    dom.borrow_mut()
        .insert_before(parent, row.marker, Some(before));
    for node in &row.fragment.nodes {
        dom.borrow_mut().insert_before(parent, *node, Some(before));
    }
}

fn create<D: Dom, K, V: Clone + 'static>(
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

fn remove<D: Dom, K, V: 'static>(dom: &Rc<RefCell<D>>, row: Row<K, V>) {
    row.fragment.remove(dom);
    dom.borrow_mut().remove(row.marker);
    row.index.dispose();
    row.item.dispose();
}
