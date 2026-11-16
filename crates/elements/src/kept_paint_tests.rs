use super::PaintedFrom;
use crate::changed_slots::ChangedSlots;

/// An update that built, freed and moved nothing.
fn unchanged() -> ChangedSlots {
    let mut changed = ChangedSlots::all();
    changed.every = false;
    changed.rebuilt.clear();
    changed.shape_changed = false;
    changed
}

#[test]
fn an_update_that_changed_nothing_keeps_the_paint() {
    // Given
    let from = PaintedFrom::default();

    // Then
    assert!(from.shows(&PaintedFrom::default(), &unchanged()));
}

#[test]
fn a_framed_rows_start_change_keeps_no_paint() {
    // Given: a framed lane whose rows from slot 2 on start elsewhere,
    // which keeping would skip (the lane bases, paint_frames.rs)
    let from = PaintedFrom::default();
    let mut changed = unchanged();
    changed.started.push((1, 2));

    // Then
    assert!(!from.shows(&PaintedFrom::default(), &changed));
}
