//! Helpers that were in `tests/value_color.rs`.
#![allow(dead_code)]

use vapors_css::value::{Color, parse_color_text};
use vapors_scene::Color as Rgba;

pub fn rgb_near(got: Rgba, r: u8, g: u8, b: u8, label: &str) {
    let [gr, gg, gb, _] = got.to_rgba8();
    for (name, got, want) in [("R", gr, r), ("G", gg, g), ("B", gb, b)] {
        let diff = i32::from(got) - i32::from(want);
        assert!(diff.abs() <= 3, "{label}: {name} = {got}, want ~{want}");
    }
}

pub fn srgb(text: &str) -> Rgba {
    match parse_color_text(text) {
        Ok(Color::Rgba(c)) => c,
        Ok(other) => panic!("{text}: not an sRGB colour: {other:?}"),
        Err(e) => panic!("{text}: {e}"),
    }
}
