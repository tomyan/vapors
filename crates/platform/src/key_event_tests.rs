//! A key as the DOM names it (3-E2a): `key` is the layout's character or
//! the named key's DOM name, Space being `" "`, with the modifiers held.

use winit::event::ElementState;
use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, NativeKeyCode, PhysicalKey};

use super::key_event;
use crate::event::{Event, Modifiers};

fn named(key: NamedKey) -> Key {
    Key::Named(key)
}

#[test]
fn named_keys_carry_their_dom_names() {
    // Then
    for key in [
        (NamedKey::Tab, "Tab", KeyCode::Tab, "Tab"),
        (NamedKey::Enter, "Enter", KeyCode::Enter, "Enter"),
        (NamedKey::Space, " ", KeyCode::Space, "Space"),
        (NamedKey::Escape, "Escape", KeyCode::Escape, "Escape"),
        (
            NamedKey::ArrowLeft,
            "ArrowLeft",
            KeyCode::ArrowLeft,
            "ArrowLeft",
        ),
        (NamedKey::Shift, "Shift", KeyCode::ShiftLeft, "ShiftLeft"),
        // winit's names that differ from the DOM's (review-v81, 7)
        (NamedKey::Super, "Meta", KeyCode::SuperLeft, "MetaLeft"),
    ] {
        let (key, dom, code, dom_code) = (key.0, key.1, key.2, key.3);
        let event = key_event(
            &named(key),
            PhysicalKey::Code(code),
            ElementState::Pressed,
            false,
            ModifiersState::empty(),
        );
        assert_eq!(
            event,
            Event::Key {
                key: dom.to_owned(),
                code: dom_code.to_owned(),
                pressed: true,
                repeat: false,
                modifiers: Modifiers::default()
            },
            "{key:?}"
        );
    }
}

#[test]
fn a_character_key_is_its_text_and_a_release_is_not_pressed() {
    // When
    let event = key_event(
        &Key::Character("a".into()),
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Released,
        false,
        ModifiersState::empty(),
    );

    // Then
    assert_eq!(
        event,
        Event::Key {
            key: "a".to_owned(),
            code: "KeyA".to_owned(),
            pressed: false,
            repeat: false,
            modifiers: Modifiers::default()
        }
    );
}

#[test]
fn the_modifiers_held_come_with_the_key() {
    // When: Shift+Tab, repeating
    let event = key_event(
        &named(NamedKey::Tab),
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
        true,
        ModifiersState::SHIFT | ModifiersState::CONTROL,
    );

    // Then
    let Event::Key {
        modifiers, repeat, ..
    } = event
    else {
        panic!("a key")
    };
    assert!(repeat);
    assert_eq!(
        modifiers,
        Modifiers {
            shift: true,
            control: true,
            alt: false,
            meta: false
        }
    );
}

#[test]
fn a_keys_code_is_its_place_whatever_shift_does_to_its_name() {
    // When: H down, then up as "H" with Shift now held
    let down = key_event(
        &Key::Character("h".into()),
        PhysicalKey::Code(KeyCode::KeyH),
        ElementState::Pressed,
        false,
        ModifiersState::empty(),
    );
    let up = key_event(
        &Key::Character("H".into()),
        PhysicalKey::Code(KeyCode::KeyH),
        ElementState::Released,
        false,
        ModifiersState::SHIFT,
    );

    // Then
    let code = |event: &Event| match event {
        Event::Key { code, .. } => code.clone(),
        _ => panic!("a key"),
    };
    assert_eq!(
        (code(&down), code(&up)),
        ("KeyH".to_owned(), "KeyH".to_owned())
    );
}

#[test]
fn two_keys_with_no_known_place_keep_apart() {
    // When
    let code = |native| match key_event(
        &Key::Unidentified(winit::keyboard::NativeKey::Unidentified),
        PhysicalKey::Unidentified(native),
        ElementState::Pressed,
        false,
        ModifiersState::empty(),
    ) {
        Event::Key { code, .. } => code,
        _ => panic!("a key"),
    };

    // Then
    assert_ne!(code(NativeKeyCode::Xkb(10)), code(NativeKeyCode::Xkb(11)));
}
