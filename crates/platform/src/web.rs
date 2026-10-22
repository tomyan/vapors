//! The page: the canvas to draw on, the status element to write to, the
//! Open button that opens the file picker from a real click.

use wasm_bindgen::prelude::*;
use web_sys::{Element, HtmlCanvasElement, HtmlElement};
use winit::event_loop::EventLoopProxy;

use crate::PlatformError;
use crate::window::Message;

/// The element id the canvas must have.
pub const CANVAS_ID: &str = "vapors";
/// The element id status text is written to, when the page has one.
pub const STATUS_ID: &str = "status";
/// The element id of the button that opens the file picker.
pub const OPEN_ID: &str = "open";

/// Whether `navigator.gpu` exists, read through `Reflect` so no
/// unstable web-sys API is needed.
pub fn has_webgpu() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    js_sys::Reflect::get(&window, &JsValue::from_str("navigator")).is_ok_and(|navigator| {
        js_sys::Reflect::has(&navigator, &JsValue::from_str("gpu")).unwrap_or(false)
    })
}

/// Make `<button id="open">` open the picker from its own click, which
/// every browser counts as user activation; a tap on the canvas does
/// not always. A page without the button is told so. The click closure
/// lives as long as the page.
pub fn wire_open_button(proxy: EventLoopProxy<Message>) {
    let Some(button) = element(OPEN_ID).and_then(|e| e.dyn_into::<HtmlElement>().ok()) else {
        crate::report::report_error(&format!(
            "the page has no <button id=\"{OPEN_ID}\">; touch screens cannot open files without it"
        ));
        return;
    };
    let on_click = Closure::<dyn FnMut()>::new(move || crate::picker::open(proxy.clone()));
    button.set_onclick(Some(on_click.as_ref().unchecked_ref()));
    on_click.forget();
}

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
