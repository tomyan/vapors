//! Wheel and trackpad scrolls: CSS pixels, signed as the DOM's deltaX
//! and deltaY, a wheel notch being `LINE_PIXELS`.

use super::*;
use crate::event::LINE_PIXELS;
use winit::dpi::PhysicalPosition;
use winit::event::{DeviceId, MouseScrollDelta, TouchPhase};

fn wheel(delta: MouseScrollDelta) -> WindowEvent {
    WindowEvent::MouseWheel {
        device_id: DeviceId::dummy(),
        delta,
        phase: TouchPhase::Moved,
    }
}

fn scrolled(scale: f64, delta: MouseScrollDelta) -> Vec<Event> {
    let mut input = Input::new();
    input.set_scale(scale);
    input.translate(&wheel(delta))
}

#[test]
fn a_notch_toward_the_user_scrolls_down_by_a_line_at_any_scale() {
    // Given: winit's line delta is negative toward the user
    for scale in [1.0, 2.0] {
        // When
        let events = scrolled(scale, MouseScrollDelta::LineDelta(0.0, -1.0));

        // Then
        assert_eq!(
            events,
            [Event::Scroll {
                dx: 0.0,
                dy: LINE_PIXELS
            }],
            "at {scale}x"
        );
    }
}

#[test]
fn a_sideways_line_scrolls_right_when_winit_says_left() {
    // When
    let events = scrolled(1.0, MouseScrollDelta::LineDelta(-2.0, 0.0));

    // Then
    assert_eq!(
        events,
        [Event::Scroll {
            dx: 2.0 * LINE_PIXELS,
            dy: 0.0
        }]
    );
}

#[test]
fn a_pixel_delta_is_divided_by_the_scale_and_negated() {
    // Given: 30 physical pixels away from the user at 2x
    let delta = MouseScrollDelta::PixelDelta(PhysicalPosition::new(10.0, 30.0));

    // When
    let events = scrolled(2.0, delta);

    // Then: 15 CSS px up, 5 left
    assert_eq!(
        events,
        [Event::Scroll {
            dx: -5.0,
            dy: -15.0
        }]
    );
}

#[test]
fn a_line_is_what_chromium_scrolls_for_a_notch() {
    assert_eq!(LINE_PIXELS, 120.0);
}

#[test]
fn a_wheel_the_page_placed_moves_the_pointer_there_first() {
    // Given: the page recorded the wheel's own position, no pointer move before
    let mut input = Input::new();
    crate::wheel_position::record([30.0, 40.0]);

    // When
    let events = input.translate(&wheel(MouseScrollDelta::LineDelta(0.0, -1.0)));

    // Then
    assert_eq!(
        events,
        [
            Event::PointerMoved { x: 30.0, y: 40.0 },
            Event::Scroll {
                dx: 0.0,
                dy: LINE_PIXELS
            }
        ]
    );
}

#[test]
fn a_wheel_the_page_placed_is_the_pointers_position_for_what_follows() {
    // Given
    let mut input = Input::new();
    input.size = Some([200.0, 200.0]);
    crate::wheel_position::record([30.0, 40.0]);
    input.translate(&wheel(MouseScrollDelta::LineDelta(0.0, -1.0)));

    // When: a pinch is anchored at the pointer
    let events = input.pinch(0.5).into_iter().collect::<Vec<_>>();

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

#[test]
fn a_native_wheel_has_no_position_of_its_own_and_sends_only_the_scroll() {
    // Given: winit's MouseWheel carries none, and nothing recorded one;
    // natively the CursorMoved winit sends places the pointer
    let mut input = Input::new();

    // When
    let events = input.translate(&wheel(MouseScrollDelta::LineDelta(0.0, -1.0)));

    // Then
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0], Event::Scroll { .. }));
}
