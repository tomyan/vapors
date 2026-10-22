//! Where failures and status go. Native: fatal errors return from `run`,
//! other failures go to stderr, status lines to stdout. Web: `run` has
//! returned by the time most things happen, so failures go to the console
//! and, with the status, into the page's `status` element when it has one.

use crate::PlatformError;

/// A failure that does not end the app: say so where the user can see.
#[cfg(not(target_family = "wasm"))]
pub fn report_error(message: &str) {
    eprintln!("vapors: {message}");
}

#[cfg(not(target_family = "wasm"))]
pub fn fatal(_error: &PlatformError) {}

/// A line that is not a failure, such as what was just opened: stdout
/// natively, the `status` element on the web. It replaces the last one.
#[cfg(not(target_family = "wasm"))]
pub fn report_status(text: &str) {
    println!("{text}");
}

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

/// A line that is not a failure, such as what was just opened: stdout
/// natively, the `status` element on the web. It replaces the last one.
#[cfg(target_family = "wasm")]
pub fn report_status(text: &str) {
    crate::web::set_status(text);
}
