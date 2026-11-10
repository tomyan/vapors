//! Where the last browser wheel event happened. winit's `MouseWheel`
//! has no position and its web backend sends no `CursorMoved` for a
//! wheel, so a wheel before any pointer movement (the pointer resting
//! where the page loaded) would be placed at no position. The web
//! (`wheel_position_web`) records each wheel event's own coordinates
//! here; `Input` takes them when it translates the `MouseWheel` that
//! follows, as a `PointerMoved` sent first. Natively nothing records
//! one, and a wheel is placed by the `CursorMoved` winit sends when the
//! pointer enters or moves over the window.

use std::cell::Cell;

thread_local! {
    static AT: Cell<Option<[f32; 2]>> = const { Cell::new(None) };
    static CANVAS_REPORTED: Cell<bool> = const { Cell::new(false) };
    static INSTALLED: Cell<bool> = const { Cell::new(false) };
}

/// A wheel event's `offsetX`/`offsetY` (CSS pixels from the canvas's
/// padding edge, in the canvas's own space, so a CSS transform, a
/// border or a sub-pixel offset is already applied) as physical pixels,
/// times `scale` physical pixels per CSS pixel: the conversion winit
/// applies to its pointer events (`event::mouse_position`).
#[allow(clippy::cast_possible_truncation)] // positions are small
#[cfg_attr(not(target_family = "wasm"), allow(dead_code))] // the web converts; tests too
pub(crate) fn canvas_position(offset: [f64; 2], scale: f64) -> [f32; 2] {
    [(offset[0] * scale) as f32, (offset[1] * scale) as f32]
}

/// Drop a recorded position that no wheel event will use: the page
/// could not place the next one.
#[cfg_attr(not(target_family = "wasm"), allow(dead_code))] // only the web clears
pub(crate) fn clear() {
    AT.take();
}

/// True the first time it is called on this thread, false after: the
/// web listener is installed once.
#[cfg_attr(not(target_family = "wasm"), allow(dead_code))] // only the web installs
pub(crate) fn first_install() -> bool {
    !INSTALLED.replace(true)
}

/// True the first time it is called on this thread, false after: a
/// page with no canvas reports it once, not on every wheel event.
#[cfg_attr(not(target_family = "wasm"), allow(dead_code))] // only the web reports
pub(crate) fn first_canvas_report() -> bool {
    !CANVAS_REPORTED.replace(true)
}

/// Record the position of the wheel event about to be translated.
#[cfg_attr(not(target_family = "wasm"), allow(dead_code))] // only the web records
pub(crate) fn record(at: [f32; 2]) {
    AT.set(Some(at));
}

/// The recorded position, once.
pub(crate) fn take() -> Option<[f32; 2]> {
    AT.take()
}

#[cfg(test)]
#[path = "wheel_position_tests.rs"]
mod tests;
