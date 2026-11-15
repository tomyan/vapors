//! A line's baseline as Blink places it (#368): the font's ascent and
//! descent rounded to device pixels, the half-leading floored to one.
//! Geist at 10 px has ascent 10.05 and descent 2.95: 10 and 3.

use crate::{Family, FontSystem, Layout, LayoutOptions, LineHeight, Style};

/// The first line's baseline and the first glyph's, device pixels from
/// the layout's top, for 10 px Geist in a `line` px line at `scale`.
fn baselines(line: f32, scale: f32) -> (f32, f32) {
    let mut system = FontSystem::bundle_only();
    let mut style = Style::new(Family::Geist, 10.0);
    style.line_height = LineHeight::Px(line);
    let options = LayoutOptions {
        scale,
        ..LayoutOptions::default()
    };
    let layout = Layout::new(&mut system, "Row 0", &style, &options);
    let line = layout.lines().next().expect("a line").baseline;
    (line, layout.glyphs().next().expect("a glyph").y)
}

#[test]
fn a_negative_half_leading_is_floored_to_a_whole_device_pixel() {
    // (12 - 10 - 3) / 2 = -0.5, floored to -1: the baseline at 10 - 1
    assert_eq!(baselines(12.0, 1.0), (9.0, 9.0));
}

#[test]
fn a_positive_half_leading_is_floored_too() {
    // (16 - 13) / 2 = 1.5, floored to 1: the baseline at 10 + 1
    assert_eq!(baselines(16.0, 1.0), (11.0, 11.0));
}

#[test]
fn at_2x_the_rounding_and_the_floor_are_in_device_pixels() {
    // ascent 20.1 and descent 5.9 round to 20 and 6; (24 - 26) / 2 = -1
    assert_eq!(baselines(12.0, 2.0), (19.0, 19.0));
}

#[test]
fn normal_line_height_puts_the_baseline_at_the_rounded_ascent() {
    // Given: normal is the rounded ascent and descent, 13 px
    let mut system = FontSystem::bundle_only();
    let mut style = Style::new(Family::Geist, 10.0);
    style.line_height = LineHeight::Normal;

    // When
    let layout = Layout::new(&mut system, "Row 0", &style, &LayoutOptions::default());

    // Then
    assert_eq!(layout.lines().next().expect("a line").baseline, 10.0);
}
