//! The pointer: a drag pans by its moves, and a pointer that left and came
//! back does not pan by the jump between where it left and where it came in.

use super::tests::view;
use super::*;

fn moved(view: &mut ImageView, x: f32, y: f32) {
    view.event(&Event::PointerMoved { x, y });
}

#[test]
fn a_drag_that_leaves_and_comes_back_does_not_pan_by_the_jump() {
    // Given: a drag that has panned by 50
    let mut view = view();
    let start = view.transform().translate;
    moved(&mut view, 100.0, 100.0);
    view.event(&Event::PrimaryButton { pressed: true });
    moved(&mut view, 150.0, 100.0);
    assert_eq!(view.transform().translate[0], start[0] + 50.0);

    // When: the pointer leaves, comes back far away, then moves by 10
    view.event(&Event::PointerLeft);
    moved(&mut view, 300.0, 300.0);
    let came_back = view.transform().translate;
    moved(&mut view, 310.0, 300.0);

    // Then
    assert_eq!(
        came_back,
        [start[0] + 50.0, start[1]],
        "no pan for the jump"
    );
    assert_eq!(view.transform().translate[0], start[0] + 60.0);
}
