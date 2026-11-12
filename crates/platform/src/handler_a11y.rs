//! The handler's part in the a11y mirror (R6): the updates the app set
//! after a frame, applied to the page's mirror in order. Natively no
//! mirror exists until R7, so an update set there stops the loop,
//! naming the fix: an app attaches a mirror on the web only.

use super::Handler;
use crate::PlatformError;

/// The page's mirror and its DOM.
#[cfg(target_family = "wasm")]
pub(crate) struct WebMirror {
    dom: crate::a11y_mirror_web::WebDom,
    mirror: crate::a11y_mirror_dom::Mirror<crate::a11y_mirror_web::WebDom>,
}

#[cfg(target_family = "wasm")]
impl WebMirror {
    /// The mirror's root on the page.
    ///
    /// # Errors
    /// The page refused the root.
    pub(crate) fn new() -> Result<Self, PlatformError> {
        let (dom, mirror) = crate::a11y_mirror_web::WebDom::mirror()?;
        Ok(Self { dom, mirror })
    }
}

impl Handler {
    /// Applies the a11y updates the app set since the last frame.
    ///
    /// # Errors
    /// An update the mirror cannot apply as given, or natively any
    /// update at all; each names the fix.
    #[cfg(target_family = "wasm")]
    pub(super) fn apply_ax_updates(&mut self) -> Result<(), PlatformError> {
        let updates = self.platform.take_ax_updates();
        let Some(web) = self.a11y.as_mut() else {
            if updates.is_empty() {
                return Ok(());
            }
            return Err(PlatformError::Windowing(
                "an a11y update arrived before the window opened".to_owned(),
            ));
        };
        for update in &updates {
            web.mirror
                .apply(&mut web.dom, update)
                .map_err(|why| PlatformError::Windowing(format!("the a11y mirror: {why}")))?;
        }
        Ok(())
    }

    #[cfg(not(target_family = "wasm"))]
    pub(super) fn apply_ax_updates(&mut self) -> Result<(), PlatformError> {
        let updates = self.platform.take_ax_updates();
        if updates.is_empty() {
            return Ok(());
        }
        Err(PlatformError::Windowing(
            "an a11y update reached the native platform, which has no a11y adapter until R7; \
             fix: call ElementsApp::mirror_a11y(true) on the web only"
                .to_owned(),
        ))
    }
}
