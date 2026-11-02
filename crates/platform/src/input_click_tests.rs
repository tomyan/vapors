//! A native press of the primary button is followed by its count in a
//! row, stamped when the platform receives it.

use std::num::NonZeroU32;

use super::*;
use winit::event::{DeviceId, ElementState, MouseButton};

fn pressing(state: ElementState) -> WindowEvent {
    WindowEvent::MouseInput {
        device_id: DeviceId::dummy(),
        state,
        button: MouseButton::Left,
    }
}

fn button(pressed: bool, n: u32) -> Event {
    Event::PrimaryButton {
        pressed,
        clicks: NonZeroU32::new(n).expect("a count"),
        modifiers: crate::event::Modifiers::default(),
    }
}

fn moved(x: f64, y: f64) -> WindowEvent {
    WindowEvent::CursorMoved {
        device_id: DeviceId::dummy(),
        position: winit::dpi::PhysicalPosition::new(x, y),
    }
}

#[test]
fn each_press_carries_its_count_and_a_release_its_presss() {
    // Given: the pointer over the window
    let mut input = Input::new();
    input.translate(&moved(10.0, 10.0));

    // When: two quick presses in one place, with their releases
    let first = input.translate(&pressing(ElementState::Pressed));
    let lifted = input.translate(&pressing(ElementState::Released));
    let second = input.translate(&pressing(ElementState::Pressed));
    let lifted_again = input.translate(&pressing(ElementState::Released));

    // Then: the count on the press itself, nothing after it
    assert_eq!(first, [button(true, 1)]);
    assert_eq!(lifted, [button(false, 1)]);
    assert_eq!(second, [button(true, 2)]);
    assert_eq!(lifted_again, [button(false, 2)]);
}

#[test]
fn a_press_with_no_known_pointer_place_starts_a_new_row() {
    // Given: a press near the origin, then the pointer leaves
    let mut input = Input::new();
    input.translate(&moved(2.0, 2.0));
    input.translate(&pressing(ElementState::Pressed));
    input.translate(&WindowEvent::CursorLeft {
        device_id: DeviceId::dummy(),
    });

    // When: a press with no place known
    let unknown = input.translate(&pressing(ElementState::Pressed));

    // Then: the first of a new row, never close to the last
    assert_eq!(unknown, [button(true, 1)]);
}

#[test]
fn a_press_carries_the_modifiers_held() {
    // Given: Alt held, as winit reports it
    let mut input = Input::new();
    input.translate(&moved(10.0, 10.0));
    input.key(Key::Modifiers(winit::keyboard::ModifiersState::ALT));

    // When
    let pressed = input.translate(&pressing(ElementState::Pressed));

    // Then
    let alt = crate::event::Modifiers {
        alt: true,
        ..crate::event::Modifiers::default()
    };
    assert_eq!(
        pressed,
        [Event::PrimaryButton {
            pressed: true,
            clicks: NonZeroU32::MIN,
            modifiers: alt
        }]
    );
}
