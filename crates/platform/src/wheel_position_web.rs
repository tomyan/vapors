//! Record each wheel event's own position (`wheel_position`). The
//! listener is on the window in the capture phase, so it runs before
//! winit's own on the canvas, and the position is there when winit's
//! `MouseWheel` is translated.

use wasm_bindgen::prelude::*;
use web_sys::WheelEvent;

use crate::wheel_position::{canvas_position, record};

/// Listen for wheel events for the page's life.
pub(crate) fn install() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let on_wheel = Closure::<dyn FnMut(WheelEvent)>::new(|event: WheelEvent| {
        let Ok(canvas) = crate::web::canvas() else {
            return;
        };
        let rect = canvas.get_bounding_client_rect();
        let scale = web_sys::window().map_or(1.0, |window| window.device_pixel_ratio());
        record(canvas_position(
            [f64::from(event.client_x()), f64::from(event.client_y())],
            [rect.left(), rect.top()],
            scale,
        ));
    });
    if let Err(error) = window.add_event_listener_with_callback_and_bool(
        "wheel",
        on_wheel.as_ref().unchecked_ref(),
        true,
    ) {
        crate::report::report_error(&format!("cannot listen for wheel events: {error:?}"));
    }
    on_wheel.forget();
}
