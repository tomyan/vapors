//! Whether a line fits a width: the one answer the line breaker and
//! the layout cache both give (`fit_tests.rs` holds them together).
//!
//! Layout hands text a width that has been through float arithmetic:
//! Taffy adds a box's padding to the text's max-content width and
//! subtracts it again for the text, which can come back one ulp
//! narrower (106.335 + 40 - 40 = 106.33499 in f32), and the width is
//! divided by the scale and multiplied back. Compared exactly, the
//! text's last word would wrap at its own max-content width. Chromium
//! keeps every length in `LayoutUnit`, fixed point at 1/64 px, where
//! that round trip is exact; a line here fits when it is at most the
//! width plus 1/64 device px. That is hundreds of ulps at UI sizes
//! (one ulp at 400 px is about 3e-5 px), so it absorbs the rounding;
//! a line it lets through overflows its box by at most 1/64 of a
//! device pixel, below what a pixel can show, and a glyph's advance is
//! normally many times that, so it moves a break only when the line
//! was already within the slack.

/// How far past a width, in device pixels, a line may reach and still
/// fit it: Chromium's `LayoutUnit` step.
pub const FIT_SLACK: f32 = 1.0 / 64.0;

/// The widest line, in device pixels, that fits `available` device
/// pixels: what the line breaker is given.
#[must_use]
pub fn fit_limit(available: f32) -> f32 {
    available + FIT_SLACK
}

/// Whether a line `content` device pixels wide fits `available`. A
/// NaN or infinite number on either side does not fit.
#[must_use]
pub fn fits(content: f32, available: f32) -> bool {
    content.is_finite() && available.is_finite() && content <= fit_limit(available)
}

#[cfg(test)]
#[path = "fit_tests.rs"]
mod tests;
