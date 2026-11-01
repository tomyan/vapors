//! The focused text field and the platform's text input (the web
//! page's hidden textarea): the app sets the field for the platform to
//! mirror after each event and frame when it changed, and hands the
//! text input's edits and pastes to the field. An edit made against a
//! text the field no longer holds is refused, reported, and the field
//! set again, so the mirror is corrected rather than the field
//! overwritten.

use vapors_elements::{Document, TextEdit};
use vapors_platform::{Event, Platform, TextField, report_error};

use super::ElementsApp;

impl ElementsApp {
    /// An edit or paste from the platform's text input, to the field.
    pub(super) fn text_event(&mut self, event: &Event) {
        match event {
            Event::TextEdited {
                before,
                text,
                selection,
                composing,
            } => {
                let edit = TextEdit {
                    before: before.clone(),
                    text: text.clone(),
                    selection: units(*selection),
                    composing: composing.map(units),
                };
                if let Err(why) = Document::text_edited(&self.document, &edit) {
                    report_error(&why);
                    self.mirrored = None;
                }
            }
            Event::PastedText(text) => {
                Document::text_pasted(&self.document, text);
            }
            _ => {}
        }
    }

    /// Sets the focused text field for the platform when it changed
    /// since it was last set.
    pub(super) fn mirror_text_field(&mut self, platform: &Platform) {
        let field = self
            .document
            .borrow_mut()
            .text_input()
            .map(|input| TextField {
                text: input.text,
                selection: (wide(input.selection.0), wide(input.selection.1)),
                area: input.area,
            });
        if self.mirrored.as_ref() != Some(&field) {
            platform.set_text_field(field.clone());
            self.mirrored = Some(field);
        }
    }
}

/// A UTF-16 range from the platform as the document counts it.
fn units((start, end): (u32, u32)) -> (usize, usize) {
    (start as usize, end as usize)
}

/// A UTF-16 count for the platform; a field's text is far below 2^32.
fn wide(units: usize) -> u32 {
    u32::try_from(units).unwrap_or(u32::MAX)
}
