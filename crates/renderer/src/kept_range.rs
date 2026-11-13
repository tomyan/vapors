//! A range of a [`Retained`](crate::Retained) buffer, held by whoever
//! kept it until it is released (`retained.rs`).

/// A range of a [`Retained`](crate::Retained) buffer, held until it is
/// given back with [`Retained::release`](crate::Retained::release); read
/// only from the buffer that kept it.
#[derive(Debug, PartialEq, Eq)]
#[must_use = "a kept range is held until it is released"]
pub struct KeptRange {
    pub(crate) owner: u64,
    pub(crate) slot: u32,
}
