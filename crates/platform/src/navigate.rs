//! Going to a followed link's address (`Platform::navigate`): the web
//! page's address is set to it; natively there is no hook to the system
//! browser yet, so it is refused, naming the address.

/// Go to `href`.
///
/// # Errors
/// The browser refusing the address, or, natively, always; the message
/// names `href` and what to do instead.
#[cfg(target_family = "wasm")]
pub(crate) fn go(href: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or_else(|| format!("no window to go to {href:?} in"))?;
    window
        .location()
        .set_href(href)
        .map_err(|error| format!("the browser refused to go to {href:?}: {error:?}"))
}

/// Go to `href`.
///
/// # Errors
/// Always natively; the message names `href` and what to do instead.
#[cfg(not(target_family = "wasm"))]
pub(crate) fn go(href: &str) -> Result<(), String> {
    Err(refusal(href))
}

/// Why a native window does not follow `href`.
#[cfg(not(target_family = "wasm"))]
fn refusal(href: &str) -> String {
    format!(
        "this window cannot go to <a href=\"{href}\">: vapors has no way to hand an address to \
         the system browser natively yet; open the page on the web, where the link sets the \
         page's address"
    )
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    #[test]
    fn a_native_window_refuses_a_link_naming_its_href() {
        // When
        let refused = super::go("?demo=counter").expect_err("no system browser hook");

        // Then
        assert!(refused.contains("<a href=\"?demo=counter\">"), "{refused}");
    }
}
