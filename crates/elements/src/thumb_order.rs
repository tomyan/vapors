//! Where a scroll container's thumb paints among its descendants (F2c
//! S9): after every one of them, as Chromium paints a scroller's
//! scrollbars after its positioned descendants, classic or overlay.
//! Measured 2026-10-25 on Chromium 155: a relative child that is not a
//! stacking context and runs under the scrollbar is painted under both
//! the classic scrollbar and the overlay thumb. A thumb stays before
//! whatever paints later and is not inside its container.

use crate::layout::LayoutBox;
use crate::paint_order::Step;

#[cfg(test)]
thread_local! {
    /// How many ancestors [`last_inside`] marked on this thread, for the
    /// test that each box is marked at most once.
    pub(crate) static MARKS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// `steps` with each thumb moved to just after the last box inside its
/// container, when that box paints later than the thumb. Linear in
/// `steps` and `boxes`: a page of scroll containers has as many thumbs
/// as boxes, and a search per thumb was quadratic.
pub(crate) fn thumbs_after_descendants(steps: &[Step], boxes: &[LayoutBox]) -> Vec<Step> {
    let last = last_inside(steps, &parents(boxes));
    let mut targets: Vec<(usize, Step)> = steps
        .iter()
        .enumerate()
        .filter_map(|(at, step)| match *step {
            Step::Thumb(container) => Some((last[container].map_or(at, |end| end.max(at)), *step)),
            Step::Box(_) | Step::Outline(_) => None,
        })
        .collect();
    // Stable: thumbs moved to one place keep their order in `steps`.
    targets.sort_by_key(|&(target, _)| target);
    placed(steps, &targets)
}

/// `steps` without their thumbs, each of `targets`' thumbs after the
/// step at its target; `targets` sorted by target.
fn placed(steps: &[Step], targets: &[(usize, Step)]) -> Vec<Step> {
    let mut out = Vec::with_capacity(steps.len());
    let mut next = targets.iter().peekable();
    for (at, step) in steps.iter().enumerate() {
        // boxes and outlines keep their places; thumbs move
        if !matches!(step, Step::Thumb(_)) {
            out.push(*step);
        }
        while let Some((_, thumb)) = next.next_if(|(target, _)| *target == at) {
            out.push(*thumb);
        }
    }
    debug_assert_eq!(
        out.len(),
        steps.len(),
        "a thumb's target is past the steps or out of order, so it was dropped: `targets` \
         must be sorted step positions (thumb_order.rs); reproduce with `cargo xtask test`"
    );
    out
}

/// For each box, the position in `steps` of the last box inside it, if
/// any. Walked from the end, each box's ancestors are marked once: an
/// ancestor already marked had its own ancestors marked by a box that
/// paints later.
fn last_inside(steps: &[Step], parents: &[Option<usize>]) -> Vec<Option<usize>> {
    let mut last = vec![None; parents.len()];
    for (at, step) in steps.iter().enumerate().rev() {
        let Step::Box(index) = *step else { continue };
        let mut above = parents[index];
        while let Some(parent) = above.filter(|&parent| last[parent].is_none()) {
            last[parent] = Some(at);
            #[cfg(test)]
            MARKS.with(|marks| marks.set(marks.get() + 1));
            above = parents[parent];
        }
    }
    last
}

fn parents(boxes: &[LayoutBox]) -> Vec<Option<usize>> {
    let mut parents = vec![None; boxes.len()];
    for (index, layout_box) in boxes.iter().enumerate() {
        for &child in &layout_box.children {
            parents[child] = Some(index);
        }
    }
    parents
}

#[cfg(test)]
#[path = "thumb_order_tests.rs"]
mod tests;
