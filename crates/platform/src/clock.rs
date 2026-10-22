//! A monotonic clock on every target. `std::time::Instant` panics on the
//! web, where `web-time` wraps `performance.now()` with the same API.

#[cfg(not(target_family = "wasm"))]
pub use std::time::Instant;

#[cfg(target_family = "wasm")]
pub use web_time::Instant;
