//! The browser's platform, from `navigator.userAgentData.platform` where
//! the browser gives it (Chromium), else `navigator.userAgent`: what the
//! click fallback (`clicks_web`) and the shortcut modifier
//! (`shortcut_modifier`) decide by.

use wasm_bindgen::JsValue;

/// `navigator.userAgentData.platform`, else `navigator.userAgent`, else
/// empty.
pub(crate) fn platform() -> String {
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
