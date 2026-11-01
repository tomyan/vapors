//! The web page's click count (E6a): each `mousedown` on the canvas
//! sends its `detail`, the browser's own count of presses in a row with
//! the system's interval, as `Event::ClickCount` after the press, which
//! winit sends from `pointerdown` (whose `detail` is 0 under Pointer
//! Events). A `detail` of 0 (a page's own synthetic `mousedown`) counts
//! as the first press.

use std::num::NonZeroU32;

use wasm_bindgen::prelude::*;
use web_sys::MouseEvent;
use winit::event_loop::EventLoopProxy;

use crate::PlatformError;
use crate::event::Event;
use crate::window::Message;

/// Listens for `mousedown` on the page's canvas for the page's life.
///
/// # Errors
/// The page has no canvas.
pub fn wire(proxy: EventLoopProxy<Message>) -> Result<(), PlatformError> {
    let canvas = crate::web::canvas()?;
    let listener = Closure::<dyn FnMut(MouseEvent)>::new(move |event: MouseEvent| {
        if event.button() != 0 {
            return;
        }
        let count = u32::try_from(event.detail()).ok().and_then(NonZeroU32::new);
        let count = count.unwrap_or(NonZeroU32::MIN);
        crate::tasks::send(&proxy, Message::Input(Event::ClickCount(count)));
    });
    drop(canvas.add_event_listener_with_callback("mousedown", listener.as_ref().unchecked_ref()));
    listener.forget();
    Ok(())
}
