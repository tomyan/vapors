//! Following a link (`Platform::navigate`, `Platform::open_new`): on the
//! web the page's address is set, or a new tab or window opened with
//! `noopener`; natively there is no hook to the system browser yet, so
//! each is refused, naming the address. A `#fragment` address is refused
//! everywhere: vapors does not scroll to an id. A detached platform (no
//! window: tests, scripted runs) refuses too, so a scripted run that
//! follows a link says so, and records the link for
//! [`Platform::take_followed`].

use crate::{Platform, PlatformError};

/// Where a new browsing context opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NewContext {
    /// A new tab (Control or Meta on a link).
    Tab,
    /// A new window (Shift on a link).
    Window,
}

/// A link a detached platform was asked to follow: its address, and the
/// new context it was to open in, `None` for in place of the page.
pub type Followed = (String, Option<NewContext>);

impl Platform {
    /// Go to `href` in place of the page: on the web, the page's address
    /// is set to it, relative to the page's.
    ///
    /// # Errors
    /// [`PlatformError::Navigation`], naming `href`: a `#fragment`;
    /// natively, always; on the web, the browser refusing it; on a
    /// detached platform, always, after recording it.
    pub fn navigate(&self, href: &str) -> Result<(), PlatformError> {
        self.follow(href, None)
    }

    /// Open `href` in a new tab or window, as Chromium does for a
    /// Control, Meta or Shift click on a link: on the web,
    /// `window.open` with `noopener`.
    ///
    /// # Errors
    /// As [`navigate`](Self::navigate).
    pub fn open_new(&self, href: &str, context: NewContext) -> Result<(), PlatformError> {
        self.follow(href, Some(context))
    }

    /// The links a detached platform was asked to follow since the last
    /// take, in order.
    #[must_use]
    pub fn take_followed(&self) -> Vec<Followed> {
        std::mem::take(&mut *self.followed.borrow_mut())
    }

    fn follow(&self, href: &str, context: Option<NewContext>) -> Result<(), PlatformError> {
        if href.starts_with('#') {
            return Err(PlatformError::Navigation(format!(
                "<a href=\"{href}\"> scrolls to an id in Chromium; vapors does not scroll to ids yet, so the link was not followed"
            )));
        }
        if self.is_detached() {
            self.followed.borrow_mut().push((href.to_owned(), context));
            return Err(PlatformError::Navigation(format!(
                "this platform is detached (no window), so <a href=\"{href}\"> was not followed; take_followed records it"
            )));
        }
        go(href, context).map_err(PlatformError::Navigation)
    }
}

/// Go to `href`, or open it in `context`.
#[cfg(target_family = "wasm")]
fn go(href: &str, context: Option<NewContext>) -> Result<(), String> {
    let window = web_sys::window().ok_or_else(|| {
        format!("no window to open <a href=\"{href}\"> in; the page must run in a browser window")
    })?;
    let refused =
        |error| format!("the browser refused <a href=\"{href}\">: {error:?}; check the address");
    match context {
        None => window.location().set_href(href).map_err(refused),
        Some(context) => {
            let features = match context {
                NewContext::Tab => "noopener",
                NewContext::Window => "noopener,popup",
            };
            // Relaxed: with `noopener` window.open always returns null, so a
            // popup the browser blocked looks the same as one it opened, and
            // a blocked link goes unreported. Accepted because noopener
            // keeps the new page from reaching this one (window.opener),
            // and the link is followed from a user's click or Enter, which
            // browsers let open. Retired by opening without noopener and
            // clearing the new window's opener ourselves, so the return
            // value tells a blocked popup apart.
            window
                .open_with_url_and_target_and_features(href, "_blank", features)
                .map(drop)
                .map_err(refused)
        }
    }
}

/// Go to `href`, or open it in `context`: refused natively.
#[cfg(not(target_family = "wasm"))]
fn go(href: &str, context: Option<NewContext>) -> Result<(), String> {
    let place = match context {
        None => "go to",
        Some(NewContext::Tab) => "open a new tab for",
        Some(NewContext::Window) => "open a new window for",
    };
    Err(format!(
        "this window cannot {place} <a href=\"{href}\">: vapors has no way to hand an address to \
         the system browser natively yet; open the page on the web, where the link opens"
    ))
}

#[cfg(all(test, not(target_family = "wasm")))]
#[path = "navigate_tests.rs"]
mod tests;
