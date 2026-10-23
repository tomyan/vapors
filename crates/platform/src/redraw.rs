//! The app's request for a frame, raised while it handles events (or by
//! a detached platform's [`Waker`](crate::Waker)) and taken by the loop
//! after them.

use std::sync::atomic::{AtomicBool, Ordering};

/// Raised by any number of requests; taken once. Atomic so a detached
/// platform's waker, which may be on another thread, can raise it.
#[derive(Default)]
pub(crate) struct RedrawFlag(AtomicBool);

impl RedrawFlag {
    pub(crate) fn raise(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Whether it was raised since the last take; lowers it.
    pub(crate) fn take(&self) -> bool {
        self.0.swap(false, Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_before_a_take_draw_one_frame_and_none_after() {
        // Given
        let flag = RedrawFlag::default();
        flag.raise();
        flag.raise();

        // When
        let (first, second) = (flag.take(), flag.take());

        // Then
        assert!(first, "two requests ask for a frame");
        assert!(!second, "a take lowers it until the next request");
    }

    #[test]
    fn no_request_draws_no_frame() {
        // Given
        let flag = RedrawFlag::default();

        // When
        let taken = flag.take();

        // Then
        assert!(!taken);
    }
}
