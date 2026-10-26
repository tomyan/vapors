//! `{for}` with keys: rows are matched by key across runs. A kept row
//! has its index and item signals updated; rows not on the longest
//! increasing subsequence of old positions are moved; new rows are
//! built and vanished rows removed.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::rc::Rc;

use vapors_signals::{Effect, Owner, untrack};

use super::rows::{Row, RowBuilder, create, remove, reuse, row_nodes};
use super::sequence::longest_increasing;
use crate::{BlockNodes, Dom, NodeId};

/// Keeps one row per `(key, item)` of `items` built before `anchor`.
/// `site` is the `{for}`'s `file:line:col`, for refusals. Returns the
/// block's current nodes, for whatever owns its position.
///
/// # Panics
/// When two items share a key, naming the key and `site`. The check
/// runs before any row is touched, so the rows stay as they were and
/// the next update applies normally.
pub fn keyed<D, K, V, I, R>(
    dom: &Rc<RefCell<D>>,
    parent: NodeId,
    anchor: NodeId,
    site: &'static str,
    items: I,
    row: R,
) -> BlockNodes
where
    D: Dom + ?Sized + 'static,
    K: Eq + Hash + Clone + Debug + 'static,
    V: PartialEq + Clone + 'static,
    I: Fn() -> Vec<(K, V)> + 'static,
    R: RowBuilder<D, V> + 'static,
{
    let dom = Rc::clone(dom);
    let block_owner = Owner::new();
    let rows: Rc<RefCell<Vec<Row<K, V>>>> = Rc::new(RefCell::new(Vec::new()));
    let state = Rc::clone(&rows);
    Effect::new(move || {
        let next = items();
        refuse_duplicate_keys(&next, site);
        let old = std::mem::take(&mut *state.borrow_mut());
        let built =
            untrack(|| block_owner.with(|| reconcile(&dom, parent, anchor, old, next, &row)));
        *state.borrow_mut() = built;
    });
    BlockNodes::new(move || {
        let mut nodes: Vec<NodeId> = rows.borrow().iter().flat_map(row_nodes).collect();
        nodes.push(anchor);
        nodes
    })
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
    D: Dom + ?Sized,
    K: Eq + Hash + Clone + 'static,
    V: PartialEq + Clone + 'static,
{
    let mut kept = by_key(old);
    let stays = stays(&old_positions(&next, &kept));
    let mut built = Vec::with_capacity(next.len());
    let mut before = anchor;
    for (index, (key, item)) in next.into_iter().enumerate().rev() {
        let row = match kept.remove(&key) {
            Some((_, existing)) => reuse(
                dom,
                parent,
                existing,
                (index, &item),
                (!stays[index]).then_some(before),
            ),
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

/// The previous rows by key, each with its old position.
fn by_key<K: Eq + Hash + Clone, V>(old: Vec<Row<K, V>>) -> HashMap<K, (usize, Row<K, V>)> {
    old.into_iter()
        .enumerate()
        .map(|(position, row)| (row.key.clone(), (position, row)))
        .collect()
}

/// For each new item, the old position of the row with its key.
fn old_positions<K: Eq + Hash, V, R>(
    next: &[(K, V)],
    kept: &HashMap<K, (usize, R)>,
) -> Vec<Option<usize>> {
    next.iter()
        .map(|(key, _)| kept.get(key).map(|(position, _)| *position))
        .collect()
}

fn refuse_duplicate_keys<K: Eq + Hash + Debug, V>(next: &[(K, V)], site: &str) {
    let mut seen = std::collections::HashSet::with_capacity(next.len());
    if let Some((key, _)) = next.iter().find(|(key, _)| !seen.insert(key)) {
        panic!(
            "{site}: two rows of this {{for}} share the key {key:?}; give `key=` an expression that is unique per row"
        );
    }
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
