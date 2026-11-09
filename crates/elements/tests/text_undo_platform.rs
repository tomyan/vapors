//! A text field's undo and redo take the platform's shortcut modifier,
//! as Chromium has them: Command+Z and Shift+Command+Z on macOS (its
//! Edit menu; Command+Y is the browser's history, not redo), Control+Z,
//! Shift+Control+Z and Control+Y elsewhere; with the modifiers held
//! exactly, so neither the other platform's modifier nor Alt undoes.
//! The app says which platform it runs on
//! (`Document::set_shortcut_modifier`).

mod support;

use support::text_page::{click, key_with, page, type_text, value};
use vapors_elements::{Document, Modifiers, ShortcutModifier};

fn control() -> Modifiers {
    Modifiers {
        control: true,
        ..Modifiers::default()
    }
}

fn command() -> Modifiers {
    Modifiers {
        meta: true,
        ..Modifiers::default()
    }
}

fn with_shift(modifiers: Modifiers) -> Modifiers {
    Modifiers {
        shift: true,
        ..modifiers
    }
}

fn with_alt(modifiers: Modifiers) -> Modifiers {
    Modifiers {
        alt: true,
        ..modifiers
    }
}

/// The text page's first field on a platform whose shortcuts take
/// `modifier`, "th" typed after "Warm".
fn typed(modifier: ShortcutModifier) -> support::text_page::Page {
    let page = page();
    page.doc.borrow_mut().set_shortcut_modifier(modifier);
    click(&page, [120.0, 12.0]);
    type_text(&page, "th");
    page
}

/// Whether `key` with `modifiers` took back "th".
fn undoes(modifier: ShortcutModifier, key: &str, modifiers: Modifiers) -> bool {
    let page = typed(modifier);
    key_with(&page, key, modifiers);
    value(&page, page.name) == "Warm"
}

/// Whether `key` with `modifiers`, after an undo, put "th" back.
fn redoes(modifier: ShortcutModifier, key: &str, modifiers: Modifiers) -> bool {
    let page = typed(modifier);
    let undo = if modifier == ShortcutModifier::Command {
        command()
    } else {
        control()
    };
    key_with(&page, "z", undo);
    key_with(&page, key, modifiers);
    value(&page, page.name) == "Warmth"
}

#[test]
fn command_z_undoes_on_macos_and_control_z_does_not() {
    assert!(undoes(ShortcutModifier::Command, "z", command()));
    assert!(!undoes(ShortcutModifier::Command, "z", control()));
}

#[test]
fn control_z_undoes_elsewhere_and_command_z_does_not() {
    assert!(undoes(ShortcutModifier::Control, "z", control()));
    assert!(!undoes(ShortcutModifier::Control, "z", command()));
}

#[test]
fn shift_command_z_redoes_on_macos_and_command_y_does_not() {
    assert!(redoes(ShortcutModifier::Command, "Z", with_shift(command())));
    assert!(!redoes(ShortcutModifier::Command, "y", command()));
    assert!(!redoes(ShortcutModifier::Command, "Z", with_shift(control())));
}

#[test]
fn shift_control_z_and_control_y_redo_elsewhere_and_command_does_not() {
    assert!(redoes(ShortcutModifier::Control, "Z", with_shift(control())));
    assert!(redoes(ShortcutModifier::Control, "y", control()));
    assert!(!redoes(ShortcutModifier::Control, "y", command()));
}

#[test]
fn alt_with_the_shortcut_is_not_undo() {
    assert!(!undoes(ShortcutModifier::Command, "z", with_alt(command())));
    assert!(!undoes(ShortcutModifier::Control, "z", with_alt(control())));
}

#[test]
fn the_field_uses_its_undo_key_and_leaves_the_other_platforms_to_the_app() {
    // Given
    let page = typed(ShortcutModifier::Command);

    // When
    let command_z = Document::key_pressed(&page.doc, "z", command());
    let control_z = Document::key_pressed(&page.doc, "z", control());

    // Then
    assert!(command_z, "the field's undo");
    assert!(!control_z, "Control+Z is the app's on macOS");
}
