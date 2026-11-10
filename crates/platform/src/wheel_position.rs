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
}

/// A wheel event's `clientX`/`clientY` as physical pixels from the
/// canvas's top-left: relative to `origin` (the canvas's client rect
/// corner), times `scale` physical pixels per CSS pixel, as winit
/// positions pointer events.
#[allow(clippy::cast_possible_truncation)] // positions are small
#[cfg_attr(not(target_family = "wasm"), allow(dead_code))] // the web converts; tests too
pub(crate) fn canvas_position(client: [f64; 2], origin: [f64; 2], scale: f64) -> [f32; 2] {
    [
        ((client[0] - origin[0]) * scale) as f32,
        ((client[1] - origin[1]) * scale) as f32,
    ]
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
