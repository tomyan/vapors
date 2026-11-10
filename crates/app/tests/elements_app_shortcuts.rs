//! `ElementsApp` tells its document which modifier the platform's
//! shortcuts take (`vapors_platform::shortcut_is_command`: Command on
//! macOS and in a browser on an Apple platform, Control elsewhere), so a
//! field's select-all follows the platform.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::ElementsApp;
use vapors_elements::{Document, ShortcutModifier};
use vapors_text::FontSystem;

#[test]
fn the_app_gives_its_document_the_platforms_shortcut_modifier() {
    // Given: a document set to the other platform's modifier
    let (expected, other) = if vapors_platform::shortcut_is_command() {
        (ShortcutModifier::Command, ShortcutModifier::Control)
    } else {
        (ShortcutModifier::Control, ShortcutModifier::Command)
    };
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.set_shortcut_modifier(other);
    let doc = Rc::new(RefCell::new(doc));

    // When
    let _app = ElementsApp::new(Rc::clone(&doc));

    // Then
    assert_eq!(doc.borrow().shortcut_modifier(), expected);
}
