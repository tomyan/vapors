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
        text: None,
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

#[test]
fn the_control_character_ctrl_v_can_produce_pastes_by_the_physical_key() {
    let control = Key::Character("\u{16}".into());
    assert!(is_paste(
        &press(&control, KeyCode::KeyV, ModifiersState::CONTROL),
        false
    ));
}

#[test]
fn the_shortcut_as_the_dom_names_it_is_the_platforms_by_the_v_place() {
    // Given
    use crate::event::Modifiers;
    use crate::paste_key::paste_shortcut;
    let control = Modifiers {
        control: true,
        ..Modifiers::default()
    };
    let command = Modifiers {
        meta: true,
        ..Modifiers::default()
    };

    // Then: Control off macOS, Command on it; a Cyrillic layout by its place
    assert!(paste_shortcut("v", "KeyV", control, false, false));
    assert!(paste_shortcut("м", "KeyV", control, false, false));
    assert!(paste_shortcut("v", "KeyV", command, false, true));
    assert!(
        !paste_shortcut("v", "KeyV", command, false, false),
        "Meta+V off macOS"
    );
    assert!(
        !paste_shortcut("v", "KeyV", control, false, true),
        "Control+V on macOS"
    );
    assert!(
        !paste_shortcut("v", "KeyV", control, true, false),
        "a repeat"
    );
}

#[test]
fn a_named_or_dead_key_at_the_v_place_is_v_by_its_place() {
    // Given
    use crate::event::Modifiers;
    use crate::paste_key::paste_shortcut;
    let control = Modifiers {
        control: true,
        ..Modifiers::default()
    };

    // Then: not a letter, so the key at the V place decides
    assert!(paste_shortcut(
        "Unidentified",
        "KeyV",
        control,
        false,
        false
    ));
    assert!(paste_shortcut("Dead", "KeyV", control, false, false));
    assert!(!paste_shortcut(
        "Unidentified",
        "KeyB",
        control,
        false,
        false
    ));
}
