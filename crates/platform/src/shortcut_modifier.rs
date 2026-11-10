//! Whether the platform's shortcut modifier is Command rather than
//! Control: Command on macOS, iPhone and iPad, Control elsewhere.
//! Natively the target OS decides; on the web the wasm target names no
//! OS, so the browser's platform decides (`browser_platform`).

/// Whether shortcuts such as select-all take Command (macOS, and a
/// browser on macOS, iPhone or iPad) rather than Control.
#[must_use]
pub fn shortcut_is_command() -> bool {
    #[cfg(target_family = "wasm")]
    return names_apple(&crate::browser_platform::platform());
    #[cfg(not(target_family = "wasm"))]
    cfg!(target_os = "macos")
}

/// Whether a browser's platform (`navigator.userAgentData.platform`,
/// else its user agent) is macOS, iPhone or iPad.
pub(crate) fn names_apple(platform: &str) -> bool {
    let platform = platform.to_ascii_lowercase();
    ["mac", "iphone", "ipad"]
        .iter()
        .any(|name| platform.contains(name))
}

#[cfg(test)]
#[path = "shortcut_modifier_tests.rs"]
mod tests;
