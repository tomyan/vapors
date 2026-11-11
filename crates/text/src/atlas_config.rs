//! How large the atlas pages are, and the sizes a device refuses.

use std::fmt;

/// Atlas sizing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtlasConfig {
    /// Side of each square page in pixels.
    pub page_size: u32,
}

impl Default for AtlasConfig {
    /// A 1024 px page: any device holds it. A painter sizes its page
    /// from its device with [`for_max_texture_side`](Self::for_max_texture_side).
    fn default() -> Self {
        Self { page_size: 1024 }
    }
}

/// The largest default page: 16 MiB for the R8 coverage page, which
/// holds the review's mixed-sizes sample at 3x (`tests/atlas_capacity.rs`).
pub const LARGEST_DEFAULT_PAGE: u32 = 4096;

/// Bytes per pixel of the widest page, the RGBA colour page.
const WIDEST_PIXEL: u32 = 4;

/// The largest side whose RGBA bytes fit a `u32`, for the refusal's
/// message; `checked` computes the overflow itself, and a test holds
/// the two together.
const LARGEST_PAGE: u32 = 32_767;

impl AtlasConfig {
    /// The default page for a device whose `max_texture_dimension_2d`
    /// is `max_texture_side`: that side, up to [`LARGEST_DEFAULT_PAGE`].
    #[must_use]
    pub fn for_max_texture_side(max_texture_side: u32) -> Self {
        Self {
            page_size: max_texture_side.min(LARGEST_DEFAULT_PAGE),
        }
    }

    /// This config, if a device whose `max_texture_dimension_2d` is
    /// `max_texture_side` can hold its pages.
    ///
    /// # Errors
    ///
    /// [`AtlasSizeError`] for a page of zero, a page over the device's
    /// side, or a page whose bytes do not fit a `u32`.
    pub fn checked(self, max_texture_side: u32) -> Result<Self, AtlasSizeError> {
        let page_size = self.page_size;
        let bytes = page_size
            .checked_mul(page_size)
            .and_then(|pixels| pixels.checked_mul(WIDEST_PIXEL));
        if page_size == 0 || page_size > max_texture_side || bytes.is_none() {
            return Err(AtlasSizeError {
                page_size,
                max_texture_side,
            });
        }
        Ok(self)
    }
}

/// An atlas page a device cannot hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtlasSizeError {
    /// The page side asked for.
    pub page_size: u32,
    /// The device's `max_texture_dimension_2d`.
    pub max_texture_side: u32,
}

impl fmt::Display for AtlasSizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            page_size,
            max_texture_side,
        } = *self;
        let largest = max_texture_side.min(LARGEST_PAGE);
        write!(
            f,
            "an atlas page of {page_size} px cannot be made: it must be at least 1 px, at most \
             this device's max_texture_dimension_2d of {max_texture_side} px, and small enough \
             that its RGBA bytes fit a u32 (at most {LARGEST_PAGE} px). Fix: give AtlasConfig a page_size \
             from 1 to {largest}; AtlasConfig::for_max_texture_side({max_texture_side}) is the default."
        )
    }
}

impl std::error::Error for AtlasSizeError {}

#[cfg(test)]
#[path = "atlas_config_tests.rs"]
mod tests;
