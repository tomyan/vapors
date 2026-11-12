//! The page: the canvas to draw on, the status element to write to, the
//! Open button that opens the file picker from a real click.

use wasm_bindgen::prelude::*;
use web_sys::{Element, HtmlCanvasElement, HtmlElement};
use winit::event_loop::EventLoopProxy;

use crate::PlatformError;
use crate::page::{Report, STATUS_ID, element_for, shows_target};
use crate::window::Message;

use crate::page::{CANVAS_ID, OPEN_ID};

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

/// Whether [`app_draws_open`] ran: the app draws its own Open and opens
/// no files through the page's button.
static APP_DRAWS_OPEN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Make `<button id="open">` send [`Message::OpenRequested`], so the app
/// opens the picker with its own file kinds. A click is user activation
/// in every browser, and it stays usable for a few seconds, long enough
/// for the event to come back through the loop; a tap on the canvas is
/// not always activation. A page without the button is told so, unless
/// the app declared [`app_draws_open`]. The click closure lives as long as the page.
pub fn wire_open_button(proxy: EventLoopProxy<Message>) {
    if APP_DRAWS_OPEN.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    let Some(button) = element(OPEN_ID).and_then(|e| e.dyn_into::<HtmlElement>().ok()) else {
        crate::report::report_error(&format!(
            "the page has no <button id=\"{OPEN_ID}\">; touch screens cannot open files without it; \
             fix: add the button, or call vapors_platform::app_draws_open() before run() if the app draws its own Open"
        ));
        return;
    };
    let on_click =
        Closure::<dyn FnMut()>::new(move || crate::tasks::send(&proxy, Message::OpenRequested));
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

/// Write `text` to the element [`element_for`] picks for `report`, if
/// the page has it; an error element is also shown (its `hidden`
/// attribute removed).
pub fn write_report(report: Report, text: &str) {
    let id = element_for(report, |id| element(id).is_some());
    let Some(target) = element(id) else {
        return;
    };
    target.set_text_content(Some(text));
    if shows_target(report) {
        drop(target.remove_attribute("hidden"));
    }
}

/// Declare that the app draws its own Open: the page needs no
/// `<button id="open">`, and one it has is hidden so it does not cover
/// the app's.
pub fn app_draws_open() {
    APP_DRAWS_OPEN.store(true, std::sync::atomic::Ordering::Relaxed);
    if let Some(button) = element(OPEN_ID) {
        drop(button.set_attribute("hidden", ""));
    }
}

/// Hide the page's status line, which also removes it from the browser's
/// accessibility tree; errors still show (see [`shows_target`]).
pub fn hide_status_line() {
    if let Some(status) = element(STATUS_ID) {
        drop(status.set_attribute("hidden", ""));
    }
}

/// The page URL's query string, `?` included, or empty.
pub fn search() -> String {
    web_sys::window()
        .and_then(|window| window.location().search().ok())
        .unwrap_or_default()
}
