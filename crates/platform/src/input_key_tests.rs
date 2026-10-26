//! `Input::key`: the modifiers kept from `ModifiersChanged`, forgotten
//! when the window loses focus, and a paste in place of its character.

use winit::event::ElementState;
use winit::keyboard::{Key as Logical, KeyCode, ModifiersState, PhysicalKey};

use super::{Input, Key};
use crate::event::Event;
use crate::paste_key::Press;

/// What follows the press's own `Event::Key`: a paste, its character, or
/// nothing.
fn press(input: &mut Input, logical: &Logical, state: ElementState) -> Vec<Event> {
    let events = input
        .key(Key::Press(Press {
            state,
            repeat: false,
            logical,
            physical: PhysicalKey::Code(KeyCode::KeyV),
            modifiers: ModifiersState::empty(),
            text: Some("v"),
        }))
        .expect("a key press is always taken");
    assert!(
        matches!(events.first(), Some(Event::Key { .. })),
        "{events:?}"
    );
    events[1..].to_vec()
}

#[test]
fn control_held_then_v_pressed_is_a_paste() {
    // Given
    let mut input = Input::new();
    let v = Logical::Character("v".into());

    // When
    let changed = input.key(Key::Modifiers(ModifiersState::CONTROL));
    let pressed = press(&mut input, &v, ElementState::Pressed);

    // Then
    assert_eq!(changed, Some(Vec::new()));
    assert_eq!(pressed, [Event::Paste]);
}

#[test]
fn v_without_control_or_released_is_not_a_paste() {
    // Given
    let mut input = Input::new();
    let v = Logical::Character("v".into());

    // When
    let alone = press(&mut input, &v, ElementState::Pressed);
    input.key(Key::Modifiers(ModifiersState::CONTROL));
    let released = press(&mut input, &v, ElementState::Released);

    // Then: its character, and nothing on the release
    assert_eq!(
        (alone, released),
        (vec![Event::Character("v".into())], Vec::new())
    );
}

#[test]
fn losing_focus_forgets_a_held_control() {
    // Given: Control held, then focus lost with no ModifiersChanged
    let mut input = Input::new();
    let v = Logical::Character("v".into());
    input.key(Key::Modifiers(ModifiersState::CONTROL));

    // When
    input.key(Key::Focus(false));
    let pressed = press(&mut input, &v, ElementState::Pressed);

    // Then
    assert_eq!(pressed, [Event::Character("v".into())]);
}

#[test]
#[ignore = "F4: text inputs. Until then a paste goes to the app whatever has focus (RELAXED in input.rs); this test is written when a focused input exists to take Ctrl+V first"]
fn paste_while_a_text_field_has_focus_goes_to_it() {
    panic!(
        "no element takes typed text yet; F4 writes this test and retires the RELAXED guard in Input::key"
    );
}
