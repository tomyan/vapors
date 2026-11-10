//! Record each wheel event's own position (`wheel_position`). The
//! listener is on the window in the capture phase, so it runs before
//! winit's own on the canvas, and the position is there when winit's
//! `MouseWheel` is translated.
//!
//! Browser coverage: the listener runs in a browser only under touche's
//! `check_idle` and `check_zoom_status`; vapors has no check of its own
//! (`docs/web.md`, "Wheel position").

use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, WheelEvent};

use crate::report::report_error;
use crate::wheel_position::{canvas_position, clear, first_canvas_report, first_install, record};

/// Listen for wheel events for the page's life.
pub(crate) fn install() {
    if !first_install() {
        report_error("the wheel listener is installed twice; install it once per page");
        return;
    }
    let Some(window) = web_sys::window() else {
        clear();
        report_error("cannot listen for wheel events: the page has no window");
        return;
    };
    let on_wheel = Closure::<dyn FnMut(WheelEvent)>::new(|event: WheelEvent| place_wheel(&event));
    if let Err(error) = window.add_event_listener_with_callback_and_bool(
        "wheel",
        on_wheel.as_ref().unchecked_ref(),
        true,
    ) {
        report_error(&format!("cannot listen for wheel events: {error:?}"));
    }
    // RELAXED: the listener is never removed, so its closure is leaked
    // with `forget`. It lives as long as the page, which has no
    // teardown; a window teardown hook that removes the listener and
    // drops the closure retires this.
    on_wheel.forget();
}

/// Record where `event` happened, as winit places a pointer event on
/// the canvas. winit's own wheel listener is on the canvas, so a wheel
/// aimed at another element is not translated and records nothing.
fn place_wheel(event: &WheelEvent) {
    clear();
    let Ok(canvas) = crate::web::canvas() else {
        if first_canvas_report() {
            report_error("cannot place a wheel event: the page has no canvas");
        }
        return;
    };
    if event
        .target()
        .as_ref()
        .and_then(|t| t.dyn_ref::<HtmlCanvasElement>())
        != Some(&canvas)
    {
        return;
    }
    let scale = web_sys::window().map_or(1.0, |window| window.device_pixel_ratio());
    record(canvas_position(offset(event), scale));
}

/// `offsetX`/`offsetY` as f64, read as winit's `mouse_position` reads
/// them: web-sys's stable getters truncate to i32.
fn offset(event: &WheelEvent) -> [f64; 2] {
    #[wasm_bindgen]
    extern "C" {
        type MouseEventExt;

        #[wasm_bindgen(method, getter, js_name = offsetX)]
        fn offset_x(this: &MouseEventExt) -> f64;

        #[wasm_bindgen(method, getter, js_name = offsetY)]
        fn offset_y(this: &MouseEventExt) -> f64;
    }

    let event: &MouseEventExt = event.unchecked_ref();
    [event.offset_x(), event.offset_y()]
}
