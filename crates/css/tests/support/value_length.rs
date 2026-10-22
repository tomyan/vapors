//! Helpers that were in `tests/value_length.rs`.
#![allow(dead_code)]

use vapors_css::value::Context;

pub fn context() -> Context {
    Context {
        font_size: 20.0,
        root_font_size: 16.0,
        viewport_width: 1000.0,
        viewport_height: 500.0,
        zero_advance: 9.0,
        percent_base: Some(200.0),
    }
}
