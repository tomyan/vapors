//! The platform's shortcut modifier has one answer: natively a document
//! no app has told takes the one `ElementsApp` would set.

use vapors_elements::ShortcutModifier;

#[test]
fn natively_an_unset_documents_shortcut_modifier_is_the_platforms() {
    assert_eq!(ShortcutModifier::default(), super::shortcut_modifier());
}
