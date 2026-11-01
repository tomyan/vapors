//! The system IME on the window (E6c): allowed while a text field has
//! focus, so composing and dead keys reach it as `Ime` events, and its
//! candidate window placed at the field's caret.

use winit::dpi::{LogicalPosition, LogicalSize};
use winit::window::Window;

use crate::TextField;

/// Allows the IME for `field`, placed at its caret, or disallows it.
pub(crate) fn allow(window: &Window, field: Option<&TextField>) {
    window.set_ime_allowed(field.is_some());
    if let Some(field) = field {
        let [x, y, width, height] = field.area;
        window.set_ime_cursor_area(
            LogicalPosition::new(f64::from(x), f64::from(y)),
            LogicalSize::new(f64::from(width), f64::from(height)),
        );
    }
}
