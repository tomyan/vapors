//! A gesture outside the area answers `OutsideArea` for every event of
//! it (down, moves, release, last lift), and a position the view does
//! not know counts as outside; a finger that starts while listed down
//! lost its end and decides afresh.

use std::num::NonZeroU32;

use vapors_platform::{Event, LINE_PIXELS, TouchPhase};

use super::Answered;
use super::tests::{IN_AREA, IN_PANEL, panel_right, touch};

#[test]
fn scroll_before_any_pointer_move_moves_nothing_and_says_so() {
    // Given: winit's wheel event carries no position, and no pointer has
    // moved: the view does not move the photo on a position it does not
    // know (the lead, review-v109)
    let mut view = panel_right();
    let before = view.transform();

    // When
    let answered = view.event(&Event::Scroll {
        dx: 0.0,
        dy: -LINE_PIXELS,
    });

    // Then
    assert_eq!(
        (answered, view.transform()),
        (Answered::OutsideArea, before)
    );
}

#[test]
fn every_event_of_a_drag_from_the_inset_answers_outside_down_moves_and_up() {
    // Given
    let mut view = panel_right();
    let fitted = view.transform();
    view.event(&Event::PointerMoved {
        x: IN_PANEL[0],
        y: IN_PANEL[1],
    });

    // When: pressed in the panel, dragged into the area, released there
    let down = view.event(&Event::PrimaryButton {
        pressed: true,
        clicks: NonZeroU32::MIN,
    });
    let moves = [250.0, 100.0].map(|x| view.event(&Event::PointerMoved { x, y: 200.0 }));
    let up = view.event(&Event::PrimaryButton {
        pressed: false,
        clicks: NonZeroU32::MIN,
    });
    let after = view.event(&Event::PointerMoved { x: 90.0, y: 200.0 });

    // Then: the whole gesture is outside; the next move is the view's
    assert_eq!([down, moves[0], moves[1], up], [Answered::OutsideArea; 4]);
    assert_eq!(view.transform(), fitted);
    assert_eq!(after, Answered::Done);
}

#[test]
fn the_last_lift_of_a_touch_from_the_inset_answers_outside() {
    // Given
    let mut view = panel_right();

    // When
    touch(&mut view, TouchPhase::Started, IN_PANEL);
    let moved = touch(&mut view, TouchPhase::Moved, IN_AREA);
    let lifted = touch(&mut view, TouchPhase::Ended, IN_AREA);

    // Then
    assert_eq!([moved, lifted], [Answered::OutsideArea; 2]);
}

#[test]
fn a_press_at_a_position_the_view_does_not_know_moves_nothing() {
    // Given: no pointer position yet
    let mut view = panel_right();
    let fitted = view.transform();

    // When
    let down = view.event(&Event::PrimaryButton {
        pressed: true,
        clicks: NonZeroU32::MIN,
    });
    view.event(&Event::PointerMoved { x: 100.0, y: 200.0 });
    view.event(&Event::PointerMoved { x: 50.0, y: 200.0 });

    // Then
    assert_eq!((down, view.transform()), (Answered::OutsideArea, fitted));
}

#[test]
fn a_finger_that_starts_while_listed_down_lost_its_end_and_decides_afresh() {
    // Given: a finger landed in the panel and its end was lost
    let mut view = panel_right();
    let fitted = view.transform();
    touch(&mut view, TouchPhase::Started, IN_PANEL);

    // When: the same finger lands in the area and pans
    let landed = touch(&mut view, TouchPhase::Started, IN_AREA);
    let panned = view.event(&Event::Pan { dx: -30.0, dy: 0.0 });

    // Then
    assert_eq!([landed, panned], [Answered::Done; 2]);
    assert_eq!(view.transform().translate[0], fitted.translate[0] - 30.0);
}
