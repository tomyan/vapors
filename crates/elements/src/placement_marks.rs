//! The marks the placement walk follows (`placement_walk.rs`): each
//! slot an update changed, marked with why, the slots up its chain
//! marked `Below`, and under each slot the marked slots placed in it,
//! so the walk into a box whose content did not move visits those and
//! not its every child (M10b: a list's every row was scanned on every
//! edit, about 55 instructions a row).

use rustc_hash::FxHashMap;

use crate::layout::Written;
use crate::placement::{Changes, LaidOut, NO_SLOT, Placement};
use crate::placement_walk::Mark;

/// The root's slot, the one slot placed in no slot.
const ROOT: usize = 0;

/// What one update marked.
#[derive(Default)]
pub(crate) struct Marked {
    /// Every slot marked, for clearing the marks after the walk.
    pub(crate) slots: Vec<usize>,
    /// The marked slots placed in each slot, in the order marked.
    pub(crate) under: FxHashMap<usize, Vec<usize>>,
}

impl Marked {
    /// Notes `slot`, placed in `up`, as marked.
    ///
    /// # Panics
    /// `slot` is not the root and is placed in no slot: the walk could
    /// not reach it, so the chain to it was not patched
    /// (`Placement::patch`).
    fn note(&mut self, slot: usize, up: usize) {
        self.slots.push(slot);
        if up == NO_SLOT {
            assert!(
                slot == ROOT,
                "slot {slot} is marked and placed in no slot, so the placement walk cannot \
                 reach it: a slot built again was not chained to the slot it is placed in \
                 (placement.rs, `patch`); `cargo test -p vapors-elements` reproduces it"
            );
            return;
        }
        self.under.entry(up).or_default().push(slot);
    }
}

impl Placement {
    /// Marks the slots `changes` names, and those up each one's chain.
    pub(crate) fn mark(&mut self, changes: &Changes<'_>, boxes: &LaidOut<'_>) -> Marked {
        let written = &changes.written;
        let started = self.started_unframed(written, boxes);
        let dirty = written
            .laid_out
            .iter()
            .chain(&written.changed)
            .chain(&started);
        let mut marked = Marked::default();
        for &slot in dirty {
            self.mark_up(slot, Mark::Dirty, &mut marked);
        }
        for &slot in changes.scrolled {
            self.mark_up(slot, Mark::Scrolled, &mut marked);
        }
        for &slot in changes.rebuilt {
            self.mark_up(slot, Mark::Rebuilt, &mut marked);
        }
        marked
    }

    /// The children whose start may have changed in each lane that was
    /// not framed: placed again, their starts added back. A framed
    /// lane's are not; their frames' shifts carry the start, and a lane
    /// whose framing changes places every child (`placement_frames.rs`).
    /// A lane dropped later in the layout that noted it holds no child:
    /// the container's children from `first` on, from its box, then.
    fn started_unframed(&self, written: &Written, boxes: &LaidOut<'_>) -> Vec<usize> {
        let unframed = |&&(container, _): &&(usize, usize)| {
            !matches!(self.framing.get(&container), Some(Ok(())))
        };
        let children = |&(container, first): &(usize, usize)| {
            let held = boxes.lanes.children_from(container, first);
            let children = &boxes.boxes[container].children;
            held.unwrap_or_else(|| children_after(children, (container, first)))
        };
        written
            .started
            .iter()
            .filter(unframed)
            .flat_map(children)
            .collect()
    }

    /// Marks `slot` with `mark`, and the slots up its chain not yet
    /// marked with `Mark::Below`, each noted under the slot it is
    /// placed in the first time it is marked.
    fn mark_up(&mut self, slot: usize, mark: Mark, marked: &mut Marked) {
        let first = self.marks[slot] == Mark::None;
        if first {
            marked.note(slot, self.up[slot]);
        }
        self.marks[slot] = self.marks[slot].max(mark);
        let mut up = if first { self.up[slot] } else { NO_SLOT };
        while up != NO_SLOT && self.marks[up] == Mark::None {
            self.marks[up] = Mark::Below;
            marked.note(up, self.up[up]);
            up = self.up[up];
        }
    }
}

/// The children of `container`, `children`, from `first` on, or a panic
/// naming both when `first` is not one: a `started` entry names a child
/// of its container in the layout that wrote it, and boxes do not change
/// within a layout.
fn children_after(children: &[usize], (container, first): (usize, usize)) -> Vec<usize> {
    let at = children.iter().position(|&child| child == first);
    let at = at.unwrap_or_else(|| {
        panic!(
            "a started entry names slot {first} under slot {container}, which is not one of \
             its children (layout/lane_write.rs `write_laid`); reproduce with \
             `cargo test -p vapors-elements`"
        )
    });
    children[at..].to_vec()
}

#[cfg(test)]
#[path = "placement_marks_tests.rs"]
mod tests;
