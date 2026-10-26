//! A key as the DOM names it (3-E2a): `key` is the layout's character or
//! the named key's DOM name, Space being `" "`, with the modifiers held.

use winit::event::ElementState;
use winit::keyboard::{Key, ModifiersState, NamedKey};

use super::key_event;
use crate::event::{Event, Modifiers};

fn named(key: NamedKey) -> Key {
    Key::Named(key)
}

#[test]
fn named_keys_carry_their_dom_names() {
    // Then
    for (key, dom) in [
        (NamedKey::Tab, "Tab"),
        (NamedKey::Enter, "Enter"),
        (NamedKey::Space, " "),
        (NamedKey::Escape, "Escape"),
        (NamedKey::ArrowLeft, "ArrowLeft"),
        (NamedKey::Shift, "Shift"),
        // winit's names that differ from the DOM's (review-v81, 7)
        (NamedKey::Super, "Meta"),
    ] {
        let event = key_event(
            &named(key),
            ElementState::Pressed,
            false,
            ModifiersState::empty(),
        );
        assert_eq!(
            event,
            Event::Key {
                key: dom.to_owned(),
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
        ElementState::Released,
        false,
        ModifiersState::empty(),
    );

    // Then
    assert_eq!(
        event,
        Event::Key {
            key: "a".to_owned(),
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
