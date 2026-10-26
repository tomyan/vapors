use winit::event::ElementState;
use winit::keyboard::{Key, KeyCode, ModifiersState, PhysicalKey};

use super::*;

fn press(logical: &Key, physical: KeyCode, modifiers: ModifiersState) -> Press<'_> {
    Press {
        state: ElementState::Pressed,
        repeat: false,
        logical,
        physical: PhysicalKey::Code(physical),
        modifiers,
    }
}

fn v() -> Key {
    Key::Character("v".into())
}

#[test]
fn control_v_pastes_off_macos_and_command_v_on_it() {
    assert!(is_paste(
        &press(&v(), KeyCode::KeyV, ModifiersState::CONTROL),
        false
    ));
    assert!(is_paste(
        &press(&v(), KeyCode::KeyV, ModifiersState::SUPER),
        true
    ));
}

#[test]
fn the_other_platforms_modifier_does_not_paste() {
    assert!(!is_paste(
        &press(&v(), KeyCode::KeyV, ModifiersState::SUPER),
        false
    ));
    assert!(!is_paste(
        &press(&v(), KeyCode::KeyV, ModifiersState::CONTROL),
        true
    ));
}

#[test]
fn v_alone_or_with_shift_or_alt_too_does_not_paste() {
    for modifiers in [
        ModifiersState::empty(),
        ModifiersState::CONTROL | ModifiersState::SHIFT,
        ModifiersState::CONTROL | ModifiersState::ALT,
    ] {
        assert!(
            !is_paste(&press(&v(), KeyCode::KeyV, modifiers), false),
            "{modifiers:?}"
        );
    }
}

#[test]
fn a_release_or_a_repeat_does_not_paste() {
    let key = v();
    let mut released = press(&key, KeyCode::KeyV, ModifiersState::CONTROL);
    released.state = ElementState::Released;
    let mut repeated = press(&key, KeyCode::KeyV, ModifiersState::CONTROL);
    repeated.repeat = true;
    assert!(!is_paste(&released, false));
    assert!(!is_paste(&repeated, false));
}

#[test]
fn a_capital_v_pastes_and_another_latin_key_does_not() {
    assert!(is_paste(
        &press(
            &Key::Character("V".into()),
            KeyCode::KeyV,
            ModifiersState::CONTROL
        ),
        false
    ));
    assert!(!is_paste(
        &press(
            &Key::Character("c".into()),
            KeyCode::KeyC,
            ModifiersState::CONTROL
        ),
        false
    ));
}

#[test]
fn dvorak_follows_the_logical_v_and_cyrillic_the_physical_one() {
    // Dvorak: logical "v" on the physical Period key; physical V is "k".
    assert!(is_paste(
        &press(&v(), KeyCode::Period, ModifiersState::CONTROL),
        false
    ));
    assert!(!is_paste(
        &press(
            &Key::Character("k".into()),
            KeyCode::KeyV,
            ModifiersState::CONTROL
        ),
        false
    ));
    // Cyrillic: logical "м" on the physical V key.
    assert!(is_paste(
        &press(
            &Key::Character("м".into()),
            KeyCode::KeyV,
            ModifiersState::CONTROL
        ),
        false
    ));
}
