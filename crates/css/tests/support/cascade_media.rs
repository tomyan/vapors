//! Helpers that were in `tests/cascade_media.rs`.
#![allow(dead_code)]

use super::{chain, css, sheet};
use vapors_css::cascade::Cascade;

pub const RED: &str = "rgb(255 0 0)";
pub const BLACK: &str = "rgb(0 0 0)";

pub fn color_under(src: &str, environment: &vapors_css::cascade::Environment) -> String {
    let (tree, leaf) = chain(&["div.card"]);
    let cascade = Cascade::new().author(sheet(src));
    let style = cascade
        .compute(&tree.node(leaf), None, environment)
        .expect("computes");
    css(&style, "color")
}
