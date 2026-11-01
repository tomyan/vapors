//! The click fallback on the web: the browser's platform, from
//! `navigator.userAgentData.platform` where the browser gives it
//! (Chromium), else `navigator.userAgent`, picks Windows', macOS's or
//! else GTK's (Linux, Chrome OS, Android) stated fallback (`clicks`).

use std::time::Duration;

use wasm_bindgen::JsValue;

/// The fallback for the browser's platform.
pub(crate) fn fallback() -> (Duration, f32) {
    crate::clicks::by_platform(&platform())
}

/// `navigator.userAgentData.platform`, else `navigator.userAgent`, else
/// empty.
fn platform() -> String {
    let Some(navigator) = web_sys::window().map(|w| JsValue::from(w.navigator())) else {
        return String::new();
    };
    let get = |from: &JsValue, key: &str| js_sys::Reflect::get(from, &JsValue::from_str(key)).ok();
    get(&navigator, "userAgentData")
        .filter(|data| !data.is_undefined())
        .and_then(|data| get(&data, "platform"))
        .and_then(|p| p.as_string())
        .or_else(|| get(&navigator, "userAgent").and_then(|ua| ua.as_string()))
        .unwrap_or_default()
}
