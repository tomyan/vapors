//! A page timer (`setTimeout`) as a future, for work with a deadline
//! that must end without a frame, such as a GPU error scope that never
//! resolves. Dropping it clears the timer.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use wasm_bindgen_futures::JsFuture;

/// Resolves `Ok` when its time passes, or `Err` at once, naming why, if
/// the page would not set the timer.
pub struct Timer {
    set: Result<(i32, JsFuture), String>,
}

impl Timer {
    /// A timer that fires `after` from now.
    #[must_use]
    pub fn after(after: Duration) -> Self {
        Self {
            set: set_timeout(after),
        }
    }
}

/// The timer's handle and its firing, or why the page would not set it.
fn set_timeout(after: Duration) -> Result<(i32, JsFuture), String> {
    let window = web_sys::window().ok_or("the page has no window")?;
    let millis = i32::try_from(after.as_millis()).map_err(|error| error.to_string())?;
    let mut handle = None;
    let fired = js_sys::Promise::new(&mut |resolve, _reject| {
        handle =
            Some(window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, millis));
    });
    let handle = handle.ok_or("the promise ran no executor")?;
    let id = handle.map_err(|error| format!("setTimeout refused: {error:?}"))?;
    Ok((id, JsFuture::from(fired)))
}

impl Future for Timer {
    type Output = Result<(), String>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        match &mut self.set {
            Err(why) => Poll::Ready(Err(why.clone())),
            Ok((_, fired)) => Pin::new(fired).poll(context).map(|_| Ok(())),
        }
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        if let (Ok((id, _)), Some(window)) = (&self.set, web_sys::window()) {
            window.clear_timeout_with_handle(*id);
        }
    }
}
