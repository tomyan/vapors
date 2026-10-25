use winit::keyboard::{Key, ModifiersState};

use super::*;

fn v() -> Key {
    Key::Character("v".into())
}

#[test]
fn control_v_pastes_off_macos_and_command_v_on_it() {
    assert!(is_paste(&v(), ModifiersState::CONTROL, false));
    assert!(is_paste(&v(), ModifiersState::SUPER, true));
}

#[test]
fn the_other_platforms_modifier_does_not_paste() {
    assert!(!is_paste(&v(), ModifiersState::SUPER, false));
    assert!(!is_paste(&v(), ModifiersState::CONTROL, true));
}

#[test]
fn v_alone_or_with_shift_or_alt_too_does_not_paste() {
    for modifiers in [
        ModifiersState::empty(),
        ModifiersState::CONTROL | ModifiersState::SHIFT,
        ModifiersState::CONTROL | ModifiersState::ALT,
    ] {
        assert!(!is_paste(&v(), modifiers, false), "{modifiers:?}");
    }
}

#[test]
fn a_capital_v_from_caps_lock_pastes_and_another_key_does_not() {
    assert!(is_paste(
        &Key::Character("V".into()),
        ModifiersState::CONTROL,
        false
    ));
    assert!(!is_paste(
        &Key::Character("c".into()),
        ModifiersState::CONTROL,
        false
    ));
}
