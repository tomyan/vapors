//! Debug builds check every update that built by owners against a
//! build of the whole tree (`docs/plans/splice.md` (b)): a kept box
//! the fresh build makes differently, such as a sibling a shallow owner
//! kept though its style changed, panics naming its slot and key.
//!
//! Compared: the slot's key, content (a field's text is its run's),
//! style, natural size, number of children, image, icon, surface,
//! range, checked, text field and its marks, and the kept flag (a box
//! is kept exactly when this update did not build it). Not compared:
//! the cache, layout and hoisted list, which a fresh build does not
//! have; `layout_check.rs` compares the layout after layout runs.

use rustc_hash::FxHashSet;
use vapors_dom::NodeId;

use super::boxes::{LayoutBox, Source, build};
use super::slots::{Placed, keys};

/// Walks `placed` from slot 0 beside a fresh build of `root`'s boxes;
/// panics at the first box that differs, or whose kept flag disagrees
/// with `built`, the slots this update built.
pub(super) fn check_against_fresh(
    placed: &Placed,
    source: &Source<'_>,
    root: NodeId,
    built: &[usize],
) {
    let fresh = build(source, root);
    let fresh_keys = keys(&fresh);
    let built: FxHashSet<usize> = built.iter().copied().collect();
    let mut stack = vec![(0, 0)];
    while let Some((index, slot)) = stack.pop() {
        let (new, kept) = (&fresh[index], &placed.boxes[slot]);
        let key = placed.slots.key(slot);
        assert!(
            key == Some(fresh_keys[index]) && same(new, kept) && kept.kept != built.contains(&slot),
            "the box in slot {slot}, {key:?}, is not the box a fresh build makes: a change missed \
             a box mark, or a shallow owner kept a child it should build \
             (docs/plans/splice.md (b)); `cargo test -p vapors-elements` reproduces it"
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
        && same_paint(fresh, kept)
}

/// The fields paint reads that the box build writes. The image is the
/// one `Images` holds, so the same image is the same `Rc`.
fn same_paint(fresh: &LayoutBox, kept: &LayoutBox) -> bool {
    let same_image = match (&fresh.image, &kept.image) {
        (Some(a), Some(b)) => std::rc::Rc::ptr_eq(a, b),
        (a, b) => a.is_none() && b.is_none(),
    };
    same_image
        && fresh.icon == kept.icon
        && fresh.surface == kept.surface
        && fresh.range == kept.range
        && fresh.checked == kept.checked
        && fresh.text_field == kept.text_field
        && fresh.text_marks == kept.text_marks
}

#[cfg(test)]
#[path = "rebuild_check_tests.rs"]
mod tests;
