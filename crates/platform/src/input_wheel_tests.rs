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
