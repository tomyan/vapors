//! Restoring runtime state when user code returns or panics. Every
//! push, flag or counter set around a call into user code is undone by
//! a guard, so a panic caught by `catch_unwind` leaves the runtime as it
//! was before the call.

/// Runs its closure when dropped: at the end of the scope, or while a
/// panic unwinds through it.
pub(crate) struct Restore<F: FnOnce()>(Option<F>);

impl<F: FnOnce()> Restore<F> {
    pub fn new(restore: F) -> Restore<F> {
        Restore(Some(restore))
    }
}

impl<F: FnOnce()> Drop for Restore<F> {
    fn drop(&mut self) {
        if let Some(restore) = self.0.take() {
            restore();
        }
    }
}
