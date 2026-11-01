//! A native press of the primary button is followed by its count in a
//! row, stamped when the platform receives it.

use std::num::NonZeroU32;

use super::*;
use winit::event::{DeviceId, ElementState, MouseButton};

fn button(state: ElementState) -> WindowEvent {
    WindowEvent::MouseInput {
        device_id: DeviceId::dummy(),
        state,
        button: MouseButton::Left,
    }
}

fn count(n: u32) -> Event {
    Event::ClickCount(NonZeroU32::new(n).expect("a count"))
}

#[test]
fn each_press_is_followed_by_its_count_and_a_release_by_none() {
    // Given
    let mut input = Input::new();

    // When: two quick presses in one place, with their releases
    let first = input.translate(&button(ElementState::Pressed));
    let lifted = input.translate(&button(ElementState::Released));
    let second = input.translate(&button(ElementState::Pressed));

    // Then
    assert_eq!(first, [Event::PrimaryButton { pressed: true }, count(1)]);
    assert_eq!(lifted, [Event::PrimaryButton { pressed: false }]);
    assert_eq!(second, [Event::PrimaryButton { pressed: true }, count(2)]);
}
