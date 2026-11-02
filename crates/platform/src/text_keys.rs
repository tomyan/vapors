//! Which key presses the page's text input (the web's hidden textarea)
//! acts on itself: a printable character, which it types and reports
//! through its `input` event; any key while an IME composes (Android's
//! keyboards compose every word); and Control or Command with C, V or
//! X, whose own `copy`, `paste` and `cut` events follow. Every other key
//! is the app's: the textarea's own action (its caret, Enter, Tab) is
//! prevented, the key reaches the app as `Event::Key`, and the app's
//! field changes the mirror.

/// Whether the textarea keeps the default action of `key` (the DOM's
/// `KeyboardEvent.key`), with Control or Meta held as given, `composing`
/// while an IME composes.
// only the web has a text input; natively its tests run it
#[cfg_attr(not(target_family = "wasm"), allow(dead_code))]
pub(crate) fn textarea_keeps(key: &str, control: bool, meta: bool, composing: bool) -> bool {
    if composing || matches!(key, "Process" | "Unidentified" | "Dead") {
        return true;
    }
    if control || meta {
        return matches!(key, "c" | "C" | "v" | "V" | "x" | "X");
    }
    key.chars().count() == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn characters_composing_and_clipboard_keys_are_the_textareas_and_the_rest_the_apps() {
        for (key, control, composing, kept) in [
            ("a", false, false, true),
            (" ", false, false, true),
            ("é", false, false, true),
            ("Backspace", false, false, false),
            ("Backspace", false, true, true),
            ("Unidentified", false, false, true),
            ("Process", false, false, true),
            ("ArrowLeft", false, false, false),
            ("Enter", false, false, false),
            ("Tab", false, false, false),
            ("a", true, false, false),
            ("z", true, false, false),
            ("v", true, false, true),
            ("c", true, false, true),
            ("x", true, false, true),
        ] {
            assert_eq!(
                textarea_keeps(key, control, false, composing),
                kept,
                "{key:?} control {control} composing {composing}"
            );
        }
        assert!(textarea_keeps("v", false, true, false), "Command+V");
    }
}
