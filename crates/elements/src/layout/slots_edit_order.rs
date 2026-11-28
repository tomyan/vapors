//! The order `put_edited` puts an owner's inserted children in, the
//! heads of their runs, and where their roots land (entry 122, action
//! 49). Each inserted child goes in after its nearest previous sibling
//! with a box, so that sibling must be in place first. Of the children
//! ready, the earliest inserted goes next, as `next_in_order` chose
//! before. That loop compared each pending child with every other, so
//! bringing back thousands of rows at once was quadratic or worse. Here
//! each child's previous sibling with a box is found once and a heap
//! yields the same sequence.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use rustc_hash::FxHashMap;
use vapors_dom::NodeId;

use super::super::super::boxes::Source;
use super::super::super::reshaped::Shared;
use super::previous_box;

/// How to reproduce a failure here.
const REPRODUCE: &str = "reproduce with `cargo nextest run -p vapors-elements child_edits`";

/// An inserted child and its nearest previous sibling with a box.
pub(super) type Insert = (NodeId, Option<NodeId>);

/// The inserted children in the order they are put, and the heads of
/// their runs: those whose previous sibling with a box is not inserted.
pub(super) struct InsertOrder {
    pub(super) sequence: Vec<Insert>,
    pub(super) heads: Vec<Insert>,
}

impl InsertOrder {
    /// The order of `inserted`, given in the order they were inserted.
    ///
    /// # Panics
    /// Two inserted children with the same previous sibling with a box:
    /// `Resolver::edits` and `previous_box` disagree about which children
    /// make a box.
    pub(super) fn of(source: &Source<'_>, inserted: &[NodeId]) -> Self {
        let previous: Vec<Option<NodeId>> =
            inserted.iter().map(|&c| previous_box(source, c)).collect();
        let (waiting, ready) = classified(inserted, &previous);
        let placed = |i: usize| (inserted[i], previous[i]);
        let heads = ready.iter().map(|&Reverse(i)| placed(i)).collect();
        let sequence = drain(inserted, &waiting, ready)
            .into_iter()
            .map(placed)
            .collect();
        Self { sequence, heads }
    }
}

/// Which inserted child (by index) waits on each inserted child, and the
/// children ready from the start: their previous sibling with a box is
/// not inserted.
fn classified(
    inserted: &[NodeId],
    previous: &[Option<NodeId>],
) -> (FxHashMap<NodeId, usize>, BinaryHeap<Reverse<usize>>) {
    let index: FxHashMap<NodeId, usize> =
        inserted.iter().enumerate().map(|(i, &c)| (c, i)).collect();
    let (mut waiting, mut ready) = (FxHashMap::default(), BinaryHeap::new());
    for (i, before) in previous.iter().enumerate() {
        match before.filter(|p| index.contains_key(p)) {
            Some(p) => {
                let other = waiting.insert(p, i);
                assert!(
                    other.is_none(),
                    "inserted children {:?} and {:?} follow the same box {p:?}: `Resolver::edits` let through a child `previous_box` skips (layout/child_edits.rs, layout/slots_edit.rs); {REPRODUCE}",
                    inserted[i],
                    other.map(|o| inserted[o])
                );
            }
            None => ready.push(Reverse(i)),
        }
    }
    (waiting, ready)
}

/// The indices in put order: the earliest inserted ready child each
/// time; putting it makes the child waiting on it ready.
fn drain(
    inserted: &[NodeId],
    waiting: &FxHashMap<NodeId, usize>,
    mut ready: BinaryHeap<Reverse<usize>>,
) -> Vec<usize> {
    let mut order = Vec::with_capacity(inserted.len());
    while let Some(Reverse(i)) = ready.pop() {
        order.push(i);
        if let Some(&next) = waiting.get(&inserted[i]) {
            ready.push(Reverse(next));
        }
    }
    order
}

/// The roots put by an edit, each after the slot it follows (`None`:
/// first), so the run of children the edit changed is assembled once.
#[derive(Default)]
pub(super) struct Inserted {
    pub(super) roots: Vec<usize>,
    following: FxHashMap<Option<usize>, usize>,
}

impl Inserted {
    /// Notes `root` put right after `after`.
    ///
    /// # Panics
    /// Another root was put after the same slot.
    pub(super) fn put_after(&mut self, after: Option<usize>, root: usize) {
        let other = self.following.insert(after, root);
        assert!(
            other.is_none(),
            "roots {other:?} and {root} were both put after {after:?}: two inserted children follow the same box (layout/slots_edit_order.rs); {REPRODUCE}"
        );
        self.roots.push(root);
    }

    /// Puts the roots into `children`, the owner's after the removals, in
    /// place: only the run between the `shared` head and tail that
    /// `reshaped_by` derived changes, as every insert's place is in it.
    ///
    /// # Panics
    /// A root follows no child of that run, nor the child before it.
    pub(super) fn splice_into(&self, children: &mut Vec<usize>, shared: Shared) {
        if self.roots.is_empty() {
            return;
        }
        let run = shared.head..children.len() - shared.tail;
        let mut out = Vec::with_capacity(run.len() + self.roots.len());
        self.push_run(shared.head.checked_sub(1).map(|i| children[i]), &mut out);
        for &child in &children[run.clone()] {
            out.push(child);
            self.push_run(Some(child), &mut out);
        }
        assert_eq!(
            out.len(),
            run.len() + self.roots.len(),
            "a root follows no child of the changed run {run:?} (layout/slots_edit_order.rs, `reshaped_by`); {REPRODUCE}"
        );
        children.splice(run, out);
    }

    fn push_run(&self, mut after: Option<usize>, out: &mut Vec<usize>) {
        while let Some(&root) = self.following.get(&after) {
            out.push(root);
            after = Some(root);
        }
    }
}
