//! Worked `.wisp` components, compiled by `build.rs` with
//! `vapors_compiler::build::compile_dir("src")`. Each file under `src`
//! becomes a module here: `counter.wisp` is [`counter::Counter`].

include!(concat!(env!("OUT_DIR"), "/wisp.rs"));

/// The rows of `scroll_rows.wisp`, and how tall each is in CSS pixels.
const SCROLL_ROWS: usize = 20;
const SCROLL_ROW_HEIGHT: f32 = 20.0;
/// How many rows `scroll_rows.wisp` shows: the three its 60 px list
/// holds and one more, for a row partly in view at each end.
const SCROLL_ROWS_SHOWN: usize = 4;

/// The row at the top of `scroll_rows.wisp`'s list at offset `y`.
#[must_use]
pub fn scroll_rows_first(y: f32) -> usize {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "an offset is 0 to the list's 340 px range, so a row index"
    )]
    let row = (y / SCROLL_ROW_HEIGHT).floor() as usize;
    row.min(SCROLL_ROWS - 1)
}

/// The height of the spacer above row `first`.
#[must_use]
pub fn scroll_rows_spacer(first: usize) -> f32 {
    #[allow(clippy::cast_precision_loss, reason = "a row index below 20")]
    let rows = first as f32;
    rows * SCROLL_ROW_HEIGHT
}

/// The rows `scroll_rows.wisp` shows from `first`.
#[must_use]
pub fn scroll_rows_shown(first: usize) -> Vec<usize> {
    (first..(first + SCROLL_ROWS_SHOWN).min(SCROLL_ROWS)).collect()
}

/// A cleanup that panics, for `cleanup_panics.wisp`: the test that a
/// component dropped while its thread is panicking is not disposed.
///
/// # Panics
/// Always, naming the component.
pub fn panicking_cleanup() {
    panic!("the cleanup of CleanupPanics ran");
}
