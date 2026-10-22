//! Helpers that were in `tests/cascade_computed.rs`.
#![allow(dead_code)]

use super::{chain, css, env, sheet};
use vapors_css::cascade::Cascade;

pub const RED: &str = "rgb(255 0 0)";
pub const BLUE: &str = "rgb(0 0 255)";

pub fn color_with_ua(ua: &str, author: &str) -> String {
    let (tree, leaf) = chain(&["span.a"]);
    let cascade = Cascade::new().user_agent(sheet(ua)).author(sheet(author));
    let style = cascade
        .compute(&tree.node(leaf), None, &env())
        .expect("computes");
    css(&style, "color")
}
