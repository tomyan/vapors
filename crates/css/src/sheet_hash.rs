//! A sheet's identity for compiled winners (`build-time-styles.md`,
//! section 2, check 1): the compiler hashes each sheet a key's winners
//! point into, and the runtime hashes each sheet it registers; a
//! difference means the rule indices were read from other text, so the
//! two sides must hash alike, and both call [`sheet_hash`].
//!
//! FNV-1a, 64-bit, over the crate's version, a separator and the text:
//! fixed by its definition, so it is the same in every build, unlike
//! `std`'s `DefaultHasher`, whose algorithm is not promised.

/// The hash of a sheet's `text` as this version of vapors-css reads it.
#[must_use]
pub fn sheet_hash(text: &str) -> u64 {
    hash_with(env!("CARGO_PKG_VERSION"), text)
}

const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0000_0100_0000_01b3;

/// [`sheet_hash`] under `version`; 0xff, which no UTF-8 text holds,
/// separates the two so no split of one string matches another.
fn hash_with(version: &str, text: &str) -> u64 {
    let bytes = version.bytes().chain([0xff]).chain(text.bytes());
    bytes.fold(OFFSET, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(PRIME)
    })
}

#[cfg(test)]
#[path = "sheet_hash_tests.rs"]
mod tests;
