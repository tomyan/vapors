//! Helpers that were in `tests/value_calc.rs`.
#![allow(dead_code)]

use vapors_css::value::{Context, parse_calc_text};

pub fn context(percent_base: Option<f64>) -> Context {
    Context {
        font_size: 16.0,
        root_font_size: 16.0,
        viewport_width: 800.0,
        viewport_height: 600.0,
        zero_advance: 8.0,
        percent_base,
    }
}

pub fn eval(expr: &str, base: Option<f64>) -> f64 {
    let calc = parse_calc_text(expr).unwrap_or_else(|e| panic!("{expr}: {e}"));
    calc.evaluate(&context(base))
        .unwrap_or_else(|e| panic!("{expr}: {e}"))
}
