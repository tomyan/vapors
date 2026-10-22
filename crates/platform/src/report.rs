//! Where failures and status go. Native: fatal errors return from `run`,
//! other failures go to stderr, status is not shown. Web: `run` has
//! returned by the time most things happen, so everything goes to the
//! console and, with the status, into the page's `status` element when it
//! has one.

use crate::PlatformError;

/// A failure that does not end the app: say so where the user can see.
#[cfg(not(target_family = "wasm"))]
pub fn report_error(message: &str) {
    eprintln!("vapors: {message}");
}

#[cfg(not(target_family = "wasm"))]
pub fn fatal(_error: &PlatformError) {}

#[cfg(not(target_family = "wasm"))]
pub fn status(_text: &str) {}

/// A failure that does not end the app: say so where the user can see.
#[cfg(target_family = "wasm")]
pub fn report_error(message: &str) {
    web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(message));
    crate::web::set_status(message);
}

#[cfg(target_family = "wasm")]
pub fn fatal(error: &PlatformError) {
    report_error(&error.to_string());
}

#[cfg(target_family = "wasm")]
pub fn status(text: &str) {
    crate::web::set_status(text);
}
