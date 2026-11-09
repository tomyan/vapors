//! The browser's platform, from `navigator.userAgentData.platform` where
//! the browser gives it (Chromium), else `navigator.userAgent`: what the
//! click fallback (`clicks_web`) and the shortcut modifier
//! (`shortcut_modifier`) decide by.

#[cfg(target_family = "wasm")]
use wasm_bindgen::JsValue;

/// `navigator.userAgentData.platform`, else `navigator.userAgent`, else
/// empty.
#[cfg(target_family = "wasm")]
pub(crate) fn platform() -> String {
    let Some(navigator) = web_sys::window().map(|w| JsValue::from(w.navigator())) else {
        return String::new();
    };
    let get = |from: &JsValue, key: &str| js_sys::Reflect::get(from, &JsValue::from_str(key)).ok();
    let hinted = get(&navigator, "userAgentData")
        .filter(|data| !data.is_undefined())
        .and_then(|data| get(&data, "platform"))
        .and_then(|p| p.as_string());
    let agent = || get(&navigator, "userAgent").and_then(|ua| ua.as_string());
    chosen(hinted, agent)
}

/// The hinted platform, else the user agent, else empty. Client Hints
/// allow an empty platform when it is unknown; that falls to the agent.
// natively its tests run it; only the web reads a browser's platform
#[cfg_attr(not(target_family = "wasm"), allow(dead_code))]
pub(crate) fn chosen(hinted: Option<String>, agent: impl FnOnce() -> Option<String>) -> String {
    hinted
        .filter(|p| !p.is_empty())
        .or_else(agent)
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "browser_platform_tests.rs"]
mod tests;
