//! The system clipboard's text, for a paste into a text field (E6b).

use crate::clipboard_reader::Answer;

/// What the clipboard held when asked for text: three answers, so a
/// clipboard that cannot be read is never taken for an empty one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardText {
    /// Text.
    Text(String),
    /// The clipboard holds no text (empty, or an image only).
    NoText,
    /// The clipboard could not be read; why.
    Unreadable(String),
}

impl Answer for ClipboardText {
    fn unreadable(why: String) -> Self {
        ClipboardText::Unreadable(why)
    }
}

/// The clipboard's text, read through arboard.
pub(crate) fn read() -> ClipboardText {
    let text = arboard::Clipboard::new().and_then(|mut clipboard| clipboard.get_text());
    match text {
        Ok(text) => ClipboardText::Text(text),
        Err(arboard::Error::ContentNotAvailable) => ClipboardText::NoText,
        Err(error) => ClipboardText::Unreadable(error.to_string()),
    }
}
