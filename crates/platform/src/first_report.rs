//! The first report of a GPU failure that ends the window's device, shared
//! between the wgpu callback that records it, the frame loop and the app.
//! Later reports are about the same device and are dropped.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// One shared slot holding the first `T` recorded into it.
#[derive(Debug)]
pub struct FirstReport<T>(Arc<Mutex<Option<T>>>);

impl<T> Clone for FirstReport<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T> Default for FirstReport<T> {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(None)))
    }
}

impl<T: Clone> FirstReport<T> {
    /// The first report, once there has been one.
    #[must_use]
    pub fn get(&self) -> Option<T> {
        self.lock().clone()
    }

    /// Keep `report` unless one is already kept.
    pub fn record(&self, report: T) {
        self.lock().get_or_insert(report);
    }

    fn lock(&self) -> MutexGuard<'_, Option<T>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
