//! Every winit window event becomes zero or more app events: the direct
//! translations from `event.rs`, plus the gestures recognised from touches
//! and trackpad pinches, which need state (the last pointer position and
//! the fingers down) that lives here.

use winit::event::WindowEvent;

use crate::clock::Instant;
use crate::event::{Event, from_winit};
use crate::gestures::Gestures;

/// Stateful translation of winit events for one window.
pub struct Input {
    pointer: [f32; 2],
    gestures: Gestures,
    started: Instant,
}

impl Input {
    /// No fingers down, pointer at the origin.
    pub fn new() -> Self {
        Self {
            pointer: [0.0, 0.0],
            gestures: Gestures::new(),
            started: Instant::now(),
        }
    }

    /// The app events for one winit event, in the order to send them.
    #[allow(clippy::cast_possible_truncation)] // a trackpad delta is a small fraction
    pub fn translate(&mut self, event: &WindowEvent) -> Vec<Event> {
        if let WindowEvent::PinchGesture { delta, .. } = event {
            return vec![Event::Pinch {
                x: self.pointer[0],
                y: self.pointer[1],
                factor: 1.0 + *delta as f32,
            }];
        }
        let Some(direct) = from_winit(event) else {
            return Vec::new();
        };
        let mut events = vec![direct.clone()];
        match direct {
            Event::PointerMoved { x, y } => self.pointer = [x, y],
            Event::Touch { id, phase, x, y } => {
                let now = self.started.elapsed();
                events.extend(self.gestures.touch(id, phase, x, y, now));
            }
            _ => {}
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::TouchPhase;
    use winit::dpi::PhysicalPosition;
    use winit::event::{DeviceId, Touch};

    fn touch(id: u64, phase: winit::event::TouchPhase, x: f64, y: f64) -> WindowEvent {
        WindowEvent::Touch(Touch {
            device_id: DeviceId::dummy(),
            phase,
            location: PhysicalPosition::new(x, y),
            force: None,
            id,
        })
    }

    #[test]
    fn a_touch_is_sent_raw_and_then_as_its_gesture() {
        // Given
        let mut input = Input::new();
        input.translate(&touch(7, winit::event::TouchPhase::Started, 0.0, 0.0));

        // When
        let events = input.translate(&touch(7, winit::event::TouchPhase::Moved, 4.0, 3.0));

        // Then
        assert_eq!(
            events,
            [
                Event::Touch {
                    id: 7,
                    phase: TouchPhase::Moved,
                    x: 4.0,
                    y: 3.0
                },
                Event::Pan { dx: 4.0, dy: 3.0 }
            ]
        );
    }

    #[test]
    fn a_trackpad_pinch_is_anchored_at_the_last_pointer_position() {
        // Given
        let mut input = Input::new();
        input.translate(&WindowEvent::CursorMoved {
            device_id: DeviceId::dummy(),
            position: PhysicalPosition::new(30.0, 40.0),
        });

        // When
        let events = input.translate(&WindowEvent::PinchGesture {
            device_id: DeviceId::dummy(),
            delta: 0.5,
            phase: winit::event::TouchPhase::Moved,
        });

        // Then
        assert_eq!(
            events,
            [Event::Pinch {
                x: 30.0,
                y: 40.0,
                factor: 1.5
            }]
        );
    }
}
