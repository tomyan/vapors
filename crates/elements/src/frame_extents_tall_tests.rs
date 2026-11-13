//! The extents index with tall steps, and the shapes it refuses.

use vapors_scene::Rect;

use super::tests::{candidates, drawn, extents, painted_again, part, quad, row, rows};
use crate::frames::PAGE;

#[test]
fn a_view_below_a_tall_first_step_visits_only_the_steps_near_it() {
    // Given: a step from 0 to 10,000 first, then 1000 rows of 10 px
    let mut boxes = vec![part(PAGE, &[quad(0.0, 10_000.0)])];
    boxes.extend((0..1000u16).map(|at| part(PAGE, &[row(f32::from(at) * 10.0)])));
    let extents = extents(&boxes);
    let view = Rect::new(0.0, 9000.0, 100.0, 9040.0);

    // When
    let visited = extents.visited(PAGE, view);

    // Then: the tall step, and the rows within `TALL` above the view;
    // rows 899 and 904 (slots 900 and 905) touch it by the bounds' fringe
    assert!(visited < 120, "visited {visited} entries");
    assert_eq!(
        candidates(&extents, PAGE, (9000.0, 9040.0)),
        [0, 900, 901, 902, 903, 904, 905]
    );
}

#[test]
#[should_panic(expected = "whose edges are not numbers")]
fn a_view_whose_edges_are_not_numbers_is_refused() {
    // Given
    let extents = extents(&rows());

    // When
    let _ = candidates(&extents, PAGE, (f32::NAN, 20.0));
}

#[test]
fn a_tall_step_patched_short_and_back_equals_the_index_made_afresh() {
    for now in [row(5.0), quad(0.0, 5000.0)] {
        // Given: slot 0 tall
        let mut boxes = rows();
        boxes[0] = part(PAGE, &[quad(0.0, 3000.0)]);
        let mut patched = extents(&boxes);

        // When
        painted_again(&mut patched, &mut boxes, (0, part(PAGE, &[now])));

        // Then
        assert_eq!(patched, extents(&boxes), "to {now:?}");
    }
}

#[test]
#[should_panic(expected = "reached the patch without a walk")]
fn a_patch_for_paint_order_of_another_length_is_refused() {
    // Given
    let boxes = rows();
    let mut extents = extents(&boxes);
    let (steps, drawn) = drawn(&boxes);

    // When: paint order one place shorter
    extents.patch(&[], (&drawn, steps.len() - 1), &boxes);
}
