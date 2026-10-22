//! Where failures and status go. Native: fatal errors return from `run`,
//! other failures go to stderr, status lines to stdout. Web: `run` has
//! returned by the time most things happen, so failures go to the console
//! and to the page's `error` element, or its `status` element when it has
//! no `error` element; status lines go to `status` only (`page.rs`).

use std::sync::OnceLock;

use crate::PlatformError;

/// What every reported line starts with, set once by `run`.
static PREFIX: OnceLock<String> = OnceLock::new();

/// The prefix for `app_name`, or for a title when the name is empty: the
/// title's first word without a trailing colon, or `vapors` when the
/// title is empty too.
pub fn prefix_from(app_name: &str, title: &str) -> String {
    let name = if app_name.trim().is_empty() {
        title
            .split_whitespace()
            .next()
            .unwrap_or("vapors")
            .trim_end_matches(':')
    } else {
        app_name.trim()
    };
    name.to_owned()
}

/// Fix the prefix for this process. The first call wins; `run` makes it.
pub fn set_prefix(app_name: &str, title: &str) {
    drop(PREFIX.set(prefix_from(app_name, title)));
}

fn prefix() -> &'static str {
    PREFIX.get().map_or("vapors", String::as_str)
}

/// A failure that does not end the app: say so where the user can see.
#[cfg(not(target_family = "wasm"))]
pub fn report_error(message: &str) {
    eprintln!("{}: {message}", prefix());
}

#[cfg(not(target_family = "wasm"))]
pub fn fatal(_error: &PlatformError) {}

/// A line that is not a failure, such as what was just opened: stdout
/// natively, the `status` element on the web. It replaces the last one.
#[cfg(not(target_family = "wasm"))]
pub fn report_status(text: &str) {
    println!("{}: {text}", prefix());
}

/// A failure that does not end the app: say so where the user can see.
#[cfg(target_family = "wasm")]
pub fn report_error(message: &str) {
    web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!(
        "{}: {message}",
        prefix()
    )));
    crate::web::write_report(crate::page::Report::Error, message);
}

#[cfg(target_family = "wasm")]
pub fn fatal(error: &PlatformError) {
    report_error(&error.to_string());
}

/// A line that is not a failure, such as what was just opened: stdout
/// natively, the `status` element on the web. It replaces the last one.
#[cfg(target_family = "wasm")]
pub fn report_status(text: &str) {
    crate::web::write_report(crate::page::Report::Status, text);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_prefix_is_the_app_name_or_the_titles_first_word() {
        // Given
        let cases = [
            ("touche", "touche: IMG_4821.jpg", "touche"),
            ("", "touche: IMG_4821.jpg", "touche"),
            ("", "vapors web demo", "vapors"),
            ("  ", "", "vapors"),
        ];

        for (app_name, title, expected) in cases {
            // When
            let prefix = prefix_from(app_name, title);

            // Then
            assert_eq!(prefix, expected, "app_name {app_name:?}, title {title:?}");
        }
    }
}
