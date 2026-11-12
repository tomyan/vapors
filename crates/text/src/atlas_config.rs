//! How large the atlas pages are, and the sizes a device refuses.

use std::fmt;

/// Atlas sizing. The coverage page starts at `page_size` and doubles,
/// up to `max_page_size`, when one frame's glyphs and icons do not fit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtlasConfig {
    /// Side of each square page in pixels when the atlas is made.
    pub page_size: u32,
    /// Largest side the coverage page grows to.
    pub max_page_size: u32,
}

impl Default for AtlasConfig {
    /// A 1024 px page that does not grow: any device holds it. A painter
    /// sizes its pages from its device with
    /// [`for_max_texture_side`](Self::for_max_texture_side).
    fn default() -> Self {
        Self::fixed(FIRST_PAGE)
    }
}

/// The side a default page starts at: 1 MiB for the R8 coverage page.
/// touche makes one painter per overlay, each with its own atlas, so a
/// page that starts at its largest side would cost that much per
/// painter (16 MiB at 4096) before any text needs it.
pub const FIRST_PAGE: u32 = 1024;

/// The largest side whose R8 coverage bytes fit a `u32` (the colour
/// page stays at most 1024, `atlas.rs`), for the refusal's message and
/// the default's cap; `checked` computes the overflow itself, and a
/// test holds the two together.
const LARGEST_PAGE: u32 = 65_535;

impl AtlasConfig {
    /// A page of `side` that does not grow.
    #[must_use]
    pub const fn fixed(side: u32) -> Self {
        Self {
            page_size: side,
            max_page_size: side,
        }
    }

    /// The default for a device whose `max_texture_dimension_2d` is
    /// `max_texture_side`: a page of [`FIRST_PAGE`] (or that side, if
    /// smaller) that grows up to that side, or 65535 px, whose bytes
    /// are the most a `u32` counts.
    #[must_use]
    pub fn for_max_texture_side(max_texture_side: u32) -> Self {
        Self {
            page_size: max_texture_side.min(FIRST_PAGE),
            max_page_size: max_texture_side.min(LARGEST_PAGE),
        }
    }

    /// This config, if a device whose `max_texture_dimension_2d` is
    /// `max_texture_side` can hold its largest page.
    ///
    /// # Errors
    ///
    /// [`AtlasSizeError`] for a page of zero, a first page over the
    /// largest, a largest page over the device's side, or a largest
    /// page whose bytes do not fit a `u32`.
    pub fn checked(self, max_texture_side: u32) -> Result<Self, AtlasSizeError> {
        let Self {
            page_size,
            max_page_size,
        } = self;
        let bytes = max_page_size.checked_mul(max_page_size);
        let refused = page_size == 0
            || page_size > max_page_size
            || max_page_size > max_texture_side
            || bytes.is_none();
        if refused {
            return Err(AtlasSizeError {
                config: self,
                max_texture_side,
            });
        }
        Ok(self)
    }
}

/// Atlas pages a device cannot hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtlasSizeError {
    /// The sizes asked for.
    pub config: AtlasConfig,
    /// The device's `max_texture_dimension_2d`.
    pub max_texture_side: u32,
}

impl fmt::Display for AtlasSizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let AtlasConfig {
            page_size,
            max_page_size,
        } = self.config;
        let side = self.max_texture_side;
        let largest = side.min(LARGEST_PAGE);
        write!(
            f,
            "atlas pages of {page_size} px growing to {max_page_size} px cannot be made: the \
             first page must be at least 1 px and at most the largest, and the largest at most \
             this device's max_texture_dimension_2d of {side} px and small enough that its \
             bytes fit a u32 (at most {LARGEST_PAGE} px). Fix: give AtlasConfig a page_size from \
             1 to max_page_size and a max_page_size up to {largest}; \
             AtlasConfig::for_max_texture_side({side}) is the default."
        )
    }
}

impl std::error::Error for AtlasSizeError {}

#[cfg(test)]
#[path = "atlas_config_tests.rs"]
mod tests;
