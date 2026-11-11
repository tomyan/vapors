//! Debug builds check every update that built by owners against a
//! build of the whole tree (`docs/plans/splice.md` (b)): a kept box
//! the fresh build makes differently, such as a sibling a shallow owner
//! kept though its style changed, panics naming its slot and key.

use vapors_dom::NodeId;

use super::boxes::{LayoutBox, Source, build};
use super::slots::Placed;

/// Walks `placed` from slot 0 beside a fresh build of `root`'s boxes;
/// panics at the first box whose content, style, natural size or
/// number of children differ.
pub(super) fn check_against_fresh(placed: &Placed, source: &Source<'_>, root: NodeId) {
    let fresh = build(source, root);
    let mut stack = vec![(0, 0)];
    while let Some((index, slot)) = stack.pop() {
        let (new, kept) = (&fresh[index], &placed.boxes[slot]);
        assert!(
            same(new, kept),
            "the box in slot {slot}, {:?}, is not the box a fresh build makes: a change missed \
             a box mark, or a shallow owner kept a child it should build \
             (docs/plans/splice.md (b)); `cargo test -p vapors-elements` reproduces it",
            placed.slots.key(slot)
        );
        stack.extend(
            new.children
                .iter()
                .copied()
                .zip(kept.children.iter().copied()),
        );
    }
}

fn same(fresh: &LayoutBox, kept: &LayoutBox) -> bool {
    fresh.content == kept.content
        && fresh.style == kept.style
        && fresh.natural == kept.natural
        && fresh.children.len() == kept.children.len()
}

#[cfg(test)]
#[path = "rebuild_check_tests.rs"]
mod tests;
