//! Helpers that were in `tests/serialize.rs`.
#![allow(dead_code)]

use super::sheet;
use vapors_css::ast::Stylesheet;

pub fn round_trip(src: &str) -> Stylesheet {
    let first = sheet(src);
    let text = first.to_css();
    match vapors_css::parse(&text) {
        Ok(s) => s,
        Err(e) => panic!("reparse of serialized form refused: {e}\n{text}"),
    }
}
