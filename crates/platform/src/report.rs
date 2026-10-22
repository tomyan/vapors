//! Where failures and status go. Native: errors return from `run` and
//! status is not shown. Web: `run` has returned by the time most things
//! happen, so errors go to the console and, with the status, into the
//! page's `status` element when it has one.

use crate::PlatformError;

#[cfg(not(target_family = "wasm"))]
pub fn error(_error: &PlatformError) {}

#[cfg(not(target_family = "wasm"))]
pub fn status(_text: &str) {}

#[cfg(target_family = "wasm")]
pub fn error(error: &PlatformError) {
    let text = error.to_string();
    web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&text));
    crate::web::set_status(&text);
}

#[cfg(target_family = "wasm")]
pub fn status(text: &str) {
    crate::web::set_status(text);
}
