//! Stateful translation: gestures, and the pointer's last position.

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

fn pinch() -> WindowEvent {
    WindowEvent::PinchGesture {
        device_id: DeviceId::dummy(),
        delta: 0.5,
        phase: winit::event::TouchPhase::Moved,
    }
}

fn resized(width: u32, height: u32) -> WindowEvent {
    WindowEvent::Resized(winit::dpi::PhysicalSize::new(width, height))
}

#[test]
fn a_trackpad_pinch_before_any_pointer_move_is_anchored_at_the_window_centre() {
    // Given
    let mut input = Input::new();
    input.translate(&resized(400, 200));

    // When
    let events = input.translate(&pinch());

    // Then
    assert_eq!(
        events,
        [Event::Pinch {
            x: 200.0,
            y: 100.0,
            factor: 1.5
        }]
    );
}

#[test]
fn a_trackpad_pinch_before_the_window_has_a_size_or_a_pointer_is_dropped() {
    assert_eq!(Input::new().translate(&pinch()), []);
}

#[test]
fn the_pointer_leaving_is_sent_and_the_next_pinch_is_anchored_at_the_centre() {
    // Given
    let mut input = Input::new();
    input.translate(&resized(400, 200));
    input.translate(&WindowEvent::CursorMoved {
        device_id: DeviceId::dummy(),
        position: PhysicalPosition::new(30.0, 40.0),
    });

    // When
    let left = input.translate(&WindowEvent::CursorLeft {
        device_id: DeviceId::dummy(),
    });
    let events = input.translate(&pinch());

    // Then
    assert_eq!(left, [Event::PointerLeft]);
    assert_eq!(
        events,
        [Event::Pinch {
            x: 200.0,
            y: 100.0,
            factor: 1.5
        }]
    );
}
