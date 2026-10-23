//! Platform input as the document's pointer calls: positions from
//! physical pixels to CSS pixels, the primary button at the last
//! position the pointer moved to, and a tap as a click. A touch is
//! taken only as its tap, so one tap is one click.

use vapors_elements::Point;
use vapors_platform::Event;

/// What one platform event asks of the document.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Action {
    Move(Point),
    Press(Point),
    Release(Point),
    Leave,
    Tap(Point),
}

/// The action for `event`, given the display `scale` and where the
/// pointer last moved (CSS pixels); `None` for input the document does
/// not take yet (keys, wheel, raw touches, pinch, pan).
pub(crate) fn action(event: &Event, scale: f32, last: Point) -> Option<Action> {
    let css = |x: f32, y: f32| [x / scale, y / scale];
    match event {
        Event::PointerMoved { x, y } => Some(Action::Move(css(*x, *y))),
        Event::PrimaryButton { pressed: true } => Some(Action::Press(last)),
        Event::PrimaryButton { pressed: false } => Some(Action::Release(last)),
        Event::PointerLeft => Some(Action::Leave),
        Event::Tap { x, y } => Some(Action::Tap(css(*x, *y))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vapors_platform::TouchPhase;

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
    fn a_tap_is_one_click_and_its_raw_touches_are_not_taken() {
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

        // Then
        assert_eq!(actions, [Action::Tap([10.0, 10.0])]);
    }
}
