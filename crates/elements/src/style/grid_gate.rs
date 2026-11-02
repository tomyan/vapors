//! CSS grid behind vapors-elements' `grid` feature (on by default): a
//! build that leaves it out (touché's page, for its size,
//! docs/design/text-stack-size.md) refuses `display: grid` and the grid
//! properties, naming the feature, rather than lay grid out as anything
//! else.
//!
//! RELAXED: the refusal in a build without the feature is tested by
//! `tests/grid_off.rs`, which runs only under
//! `cargo test -p vapors-elements --no-default-features`; `cargo xtask
//! ci` builds the default features. The ci step for no-default-features
//! builds (vapors issue #121) retires this.

use taffy::Display;

/// Why a property or display that needs grid is refused without it.
pub(crate) const GRID_OFF: &str = "needs CSS grid, which this build leaves out (vapors-elements' `grid` feature); turn the feature on, or lay it out with flex";

/// `display: grid` (or a blockified `inline-grid`): a grid container,
/// or, without the feature, refused.
// with the feature it is always Ok; the Result is the build without it
#[cfg_attr(feature = "grid", allow(clippy::unnecessary_wraps))]
pub(crate) fn grid() -> Result<Display, String> {
    #[cfg(feature = "grid")]
    return Ok(Display::Grid);
    #[cfg(not(feature = "grid"))]
    Err(format!("`display: grid` {GRID_OFF}"))
}

/// Whether a box of `display` makes its children flex or grid items.
pub(crate) fn is_container(display: Display) -> bool {
    #[cfg(feature = "grid")]
    if display == Display::Grid {
        return true;
    }
    display == Display::Flex
}
