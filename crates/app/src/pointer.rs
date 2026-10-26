//! Platform input as the document's pointer calls: positions from
//! physical pixels to CSS pixels, the primary button at the last
//! position the pointer moved to, and a tap as a click. A wheel scrolls
//! where the pointer is. A touch is a pointer of its own (3-E1b) and
//! clicks only as its tap, so one tap is one click; its down and up
//! also bound a gesture whose pans scroll.

use vapors_elements::Point;
use vapors_platform::{Event, TouchPhase};

/// What one platform event asks of the document.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Action {
    Move(Point),
    Press(Point),
    Release(Point),
    Leave,
    Tap(Point),
    /// Scroll by a delta (CSS pixels) at a point.
    Wheel(Point, [f32; 2]),
    /// A finger (by id) went down; the first starts a gesture there.
    TouchDown(u64, Point),
    /// A finger (by id) moved to a point.
    TouchMove(u64, Point),
    /// The fingers moved: scroll by this delta (CSS pixels).
    Pan([f32; 2]),
    /// A finger (by id) lifted at a point; the last ends the gesture.
    TouchUp(u64, Point),
    /// The platform took a finger (by id) at a point.
    TouchCancel(u64, Point),
}

/// The action for `event`, given the display `scale` and where the
/// pointer last moved (CSS pixels); `None` for input the document does
/// not take yet (keys, pinch).
pub(crate) fn action(event: &Event, scale: f32, last: Point) -> Option<Action> {
    let css = |x: f32, y: f32| [x / scale, y / scale];
    match event {
        Event::PointerMoved { x, y } => Some(Action::Move(css(*x, *y))),
        Event::PrimaryButton { pressed: true } => Some(Action::Press(last)),
        Event::PrimaryButton { pressed: false } => Some(Action::Release(last)),
        Event::PointerLeft => Some(Action::Leave),
        Event::Tap { x, y } => Some(Action::Tap(css(*x, *y))),
        Event::Scroll { dx, dy } => Some(Action::Wheel(last, [*dx, *dy])),
        // a finger moving right drags the content right: a scroll left
        Event::Pan { dx, dy } => Some(Action::Pan([-dx / scale, -dy / scale])),
        Event::Touch { id, phase, x, y } => Some(touch(*id, *phase, css(*x, *y))),
        _ => None,
    }
}

fn touch(id: u64, phase: TouchPhase, at: Point) -> Action {
    match phase {
        TouchPhase::Started => Action::TouchDown(id, at),
        TouchPhase::Moved => Action::TouchMove(id, at),
        TouchPhase::Ended => Action::TouchUp(id, at),
        TouchPhase::Cancelled => Action::TouchCancel(id, at),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_are_divided_by_the_display_scale() {
        // Given
        let event = Event::PointerMoved { x: 30.0, y: 50.0 };

        // When
        let found = action(&event, 2.0, [0.0, 0.0]);

        // Then
        assert_eq!(found, Some(Action::Move([15.0, 25.0])));
    }

    #[test]
    fn the_button_acts_where_the_pointer_last_moved() {
        // Given
        let (press, release) = (
            Event::PrimaryButton { pressed: true },
            Event::PrimaryButton { pressed: false },
        );

        // When
        let found = (
            action(&press, 1.0, [4.0, 5.0]),
            action(&release, 1.0, [6.0, 7.0]),
        );

        // Then
        assert_eq!(
            found,
            (
                Some(Action::Press([4.0, 5.0])),
                Some(Action::Release([6.0, 7.0]))
            )
        );
    }

    #[test]
    fn a_cancelled_touch_is_a_cancel_of_its_finger() {
        // Given
        let cancelled = Event::Touch {
            id: 3,
            phase: TouchPhase::Cancelled,
            x: 0.0,
            y: 0.0,
        };

        // When / Then
        assert_eq!(
            action(&cancelled, 1.0, [0.0, 0.0]),
            Some(Action::TouchCancel(3, [0.0, 0.0]))
        );
    }

    #[test]
    fn a_wheel_scrolls_its_css_delta_where_the_pointer_is() {
        // When
        let found = action(&Event::Scroll { dx: 0.0, dy: 120.0 }, 2.0, [4.0, 5.0]);

        // Then: deltas are CSS pixels already
        assert_eq!(found, Some(Action::Wheel([4.0, 5.0], [0.0, 120.0])));
    }

    #[test]
    fn a_pan_scrolls_against_the_fingers_in_css_pixels() {
        // When: fingers move 20 right, 40 down physical at 2x
        let found = action(&Event::Pan { dx: 20.0, dy: 40.0 }, 2.0, [0.0, 0.0]);

        // Then
        assert_eq!(found, Some(Action::Pan([-10.0, -20.0])));
    }

    #[test]
    fn a_tap_is_one_click_after_its_touchs_own_down_and_up() {
        // Given: what the platform sends for one tap
        let events = [
            Event::Touch {
                id: 1,
                phase: TouchPhase::Started,
                x: 10.0,
                y: 10.0,
            },
            Event::Touch {
                id: 1,
                phase: TouchPhase::Ended,
                x: 10.0,
                y: 10.0,
            },
            Event::Tap { x: 10.0, y: 10.0 },
        ];

        // When
        let actions: Vec<Action> = events
            .iter()
            .filter_map(|e| action(e, 1.0, [0.0, 0.0]))
            .collect();

        // Then: the touch's down and up, then the tap, which is the click
        assert_eq!(
            actions,
            [
                Action::TouchDown(1, [10.0, 10.0]),
                Action::TouchUp(1, [10.0, 10.0]),
                Action::Tap([10.0, 10.0])
            ]
        );
    }
}
