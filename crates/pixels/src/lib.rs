//! Pixel checks for a demo's screenshot. One file per demo,
//! `crates/demos/demos/<name>.pixels`, read by the native screenshot
//! (the demo example) and the web screenshot (the page-contract script,
//! through the `check_pixels` binary), so the two cannot drift apart.
//! This crate depends on `png` alone, so the web job builds the binary
//! in seconds.
//!
//! ```text
//! at WxH                       the lines after it are for a W x H CSS px viewport
//! click X Y                    before the screenshot, click there (in file order)
//! pixel X Y #rrggbb            exact, within 1 per channel
//! ink X0 Y0 X1 Y1 #rrggbb      some pixel in the box is not that colour
//! ```
//!
//! Coordinates are CSS pixels; `#` after the fields starts a comment.
//! Lines before any `at` are for the 640x400 viewport the native window
//! and the web page's screenshot use; an `at` section is checked by the
//! gpu tier, which renders at any size.

mod check;
mod parse;

pub use check::{Image, failures};
pub use parse::{Check, DEFAULT_VIEWPORT, Spec, parse, section};

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
