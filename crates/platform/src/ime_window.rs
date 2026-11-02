//! The system IME on the window (E6c): allowed while a text field has
//! focus, so composing and dead keys reach it as `Ime` events, and its
//! candidate window placed at the field's caret. A field set before the
//! window exists is kept by the composition and applied when it opens.

use winit::dpi::{LogicalPosition, LogicalSize};
use winit::window::Window;

use crate::TextField;
use crate::ime::Composition;

/// What the IME is told through: the window, or a test's stand-in.
pub(crate) trait ImeTarget {
    fn set_ime_allowed(&self, allowed: bool);
    /// The caret's rect, `[x, y]` and `[width, height]` in CSS px.
    fn set_ime_cursor_area(&self, at: [f64; 2], size: [f64; 2]);
}

impl ImeTarget for Window {
    fn set_ime_allowed(&self, allowed: bool) {
        Window::set_ime_allowed(self, allowed);
    }

    fn set_ime_cursor_area(&self, [x, y]: [f64; 2], [width, height]: [f64; 2]) {
        Window::set_ime_cursor_area(
            self,
            LogicalPosition::new(x, y),
            LogicalSize::new(width, height),
        );
    }
}

/// The app set the focused field (or none): the composition takes it,
/// and the window, once it exists, allows the IME for it. A composition
/// the new field ends is dropped by the IME too, with its focus.
pub(crate) fn mirror(
    window: Option<&impl ImeTarget>,
    composition: &mut Composition,
    field: Option<&TextField>,
) {
    let ended = composition.set_field(field.cloned());
    let Some(window) = window else {
        return;
    };
    if ended {
        window.set_ime_allowed(false);
    }
    allow(window, field);
}

/// The window opened: the IME for the field set before it, if any.
pub(crate) fn opened(window: &impl ImeTarget, composition: &Composition) {
    allow(window, composition.field());
}

/// Allows the IME for `field`, placed at its caret, or disallows it.
fn allow(window: &impl ImeTarget, field: Option<&TextField>) {
    window.set_ime_allowed(field.is_some());
    if let Some(field) = field {
        let [x, y, width, height] = field.area.map(f64::from);
        window.set_ime_cursor_area([x, y], [width, height]);
    }
}

#[cfg(test)]
#[path = "ime_window_tests.rs"]
mod tests;
