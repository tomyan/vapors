//! Where a scroll container's thumb paints among its descendants (F2c
//! S9): after every one of them, as Chromium paints a scroller's
//! scrollbars after its positioned descendants, classic or overlay.
//! Measured 2026-10-25 on Chromium 155: a relative child that is not a
//! stacking context and runs under the scrollbar is painted under both
//! the classic scrollbar and the overlay thumb. A thumb stays before
//! whatever paints later and is not inside its container.

use crate::layout::LayoutBox;
use crate::paint_order::Step;

/// `steps` with each thumb moved to just after the last box inside its
/// container, when that box paints later than the thumb.
pub(crate) fn thumbs_after_descendants(steps: &[Step], boxes: &[LayoutBox]) -> Vec<Step> {
    let parents = parents(boxes);
    let targets: Vec<(usize, Step)> = steps
        .iter()
        .enumerate()
        .filter_map(|(at, step)| match *step {
            Step::Thumb(container) => {
                Some((last_inside(steps, &parents, container).max(at), *step))
            }
            Step::Box(_) => None,
        })
        .collect();
    let mut out = Vec::with_capacity(steps.len());
    for (at, step) in steps.iter().enumerate() {
        if let Step::Box(_) = step {
            out.push(*step);
        }
        out.extend(
            targets
                .iter()
                .filter(|(target, _)| *target == at)
                .map(|(_, thumb)| *thumb),
        );
    }
    out
}

/// The position in `steps` of the last box inside `container`, or 0.
fn last_inside(steps: &[Step], parents: &[Option<usize>], container: usize) -> usize {
    let inside = |mut index: usize| {
        while let Some(parent) = parents[index] {
            if parent == container {
                return true;
            }
            index = parent;
        }
        false
    };
    steps
        .iter()
        .rposition(|step| matches!(*step, Step::Box(index) if inside(index)))
        .unwrap_or(0)
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
