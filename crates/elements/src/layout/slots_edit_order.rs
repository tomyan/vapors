//! The order `put_edited` puts an owner's inserted children in, and the
//! heads of their runs (entry 122, action 49). Each inserted child goes
//! in after its nearest previous sibling with a box, so that sibling must
//! be in place first. Of the children ready, the earliest inserted goes
//! next, as `next_in_order` chose before. That loop compared each pending
//! child with every other, so bringing back thousands of rows at once was
//! quadratic or worse. Here each child is looked at once and a heap
//! yields the same sequence.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use rustc_hash::FxHashMap;
use vapors_dom::NodeId;

use super::super::super::boxes::Source;
use super::previous_box;

/// The inserted children in the order they are put, and the heads of
/// their runs: those whose previous sibling with a box is not inserted.
pub(super) struct InsertOrder {
    pub(super) sequence: Vec<NodeId>,
    pub(super) heads: Vec<NodeId>,
}

impl InsertOrder {
    /// The order of `inserted`, given in the order they were inserted.
    ///
    /// # Panics
    /// Two inserted children with the same previous sibling with a box, or
    /// one never reached: `Resolver::edits` let through a child that makes
    /// no box.
    pub(super) fn of(source: &Source<'_>, inserted: &[NodeId]) -> Self {
        let index: FxHashMap<NodeId, usize> =
            inserted.iter().enumerate().map(|(i, &c)| (c, i)).collect();
        let mut waiting: FxHashMap<NodeId, usize> = FxHashMap::default();
        let (mut ready, mut heads) = (BinaryHeap::new(), Vec::new());
        for (i, &child) in inserted.iter().enumerate() {
            match previous_box(source, child).filter(|p| index.contains_key(p)) {
                Some(previous) => {
                    let other = waiting.insert(previous, i);
                    assert!(
                        other.is_none(),
                        "inserted children {child:?} and {:?} follow the same box {previous:?}",
                        other.map(|o| inserted[o])
                    );
                }
                None => {
                    ready.push(Reverse(i));
                    heads.push(child);
                }
            }
        }
        let sequence = drain(inserted, &waiting, ready);
        Self { sequence, heads }
    }
}

/// Takes the earliest inserted ready child each time; putting it makes the
/// child waiting on it ready.
fn drain(
    inserted: &[NodeId],
    waiting: &FxHashMap<NodeId, usize>,
    mut ready: BinaryHeap<Reverse<usize>>,
) -> Vec<NodeId> {
    let mut sequence = Vec::with_capacity(inserted.len());
    while let Some(Reverse(i)) = ready.pop() {
        sequence.push(inserted[i]);
        if let Some(&next) = waiting.get(&inserted[i]) {
            ready.push(Reverse(next));
        }
    }
    assert_eq!(
        sequence.len(),
        inserted.len(),
        "inserted children form runs after a box, so each is reached"
    );
    sequence
}

/// The roots put by an edit, each after the slot it follows (`None`:
/// first), so the owner's children are assembled in one pass.
#[derive(Default)]
pub(super) struct Inserted {
    pub(super) roots: Vec<usize>,
    following: FxHashMap<Option<usize>, usize>,
}

impl Inserted {
    /// Notes `root` put right after `after`.
    ///
    /// # Panics
    /// Another root was put after the same slot: two inserted children
    /// share a previous sibling with a box.
    pub(super) fn put_after(&mut self, after: Option<usize>, root: usize) {
        let other = self.following.insert(after, root);
        assert!(
            other.is_none(),
            "roots {other:?} and {root} were both put after {after:?}"
        );
        self.roots.push(root);
    }

    /// `children`, the owner's after the removals, with each root put in
    /// after the slot it follows, and the run of roots after it.
    pub(super) fn spliced_into(&self, children: &[usize]) -> Vec<usize> {
        let mut out = Vec::with_capacity(children.len() + self.roots.len());
        self.push_run(None, &mut out);
        for &child in children {
            out.push(child);
            self.push_run(Some(child), &mut out);
        }
        assert_eq!(
            out.len(),
            children.len() + self.roots.len(),
            "every root follows a child or a root"
        );
        out
    }

    fn push_run(&self, mut after: Option<usize>, out: &mut Vec<usize>) {
        while let Some(&root) = self.following.get(&after) {
            out.push(root);
            after = Some(root);
        }
    }
}
