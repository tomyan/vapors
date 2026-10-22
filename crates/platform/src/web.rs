//! The page: the canvas to draw on, the status element to write to.

use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlCanvasElement};

use crate::PlatformError;

/// The element id the canvas must have.
pub const CANVAS_ID: &str = "vapors";
/// The element id status text is written to, when the page has one.
pub const STATUS_ID: &str = "status";

/// Panics become `console.error` lines with a Rust message, not `unreachable`.
pub fn install_panic_hook() {
    console_error_panic_hook::set_once();
}

fn element(id: &str) -> Option<Element> {
    web_sys::window()?.document()?.get_element_by_id(id)
}

/// The page's canvas.
///
/// # Errors
///
/// [`PlatformError::Windowing`] naming the id when the page has no such canvas.
pub fn canvas() -> Result<HtmlCanvasElement, PlatformError> {
    element(CANVAS_ID)
        .and_then(|e| e.dyn_into::<HtmlCanvasElement>().ok())
        .ok_or_else(|| {
            PlatformError::Windowing(format!(
                "the page has no <canvas id=\"{CANVAS_ID}\">; add one and size it with CSS"
            ))
        })
}

/// Replace the text of the `status` element, if the page has one.
pub fn set_status(text: &str) {
    if let Some(element) = element(STATUS_ID) {
        element.set_text_content(Some(text));
    }
}
