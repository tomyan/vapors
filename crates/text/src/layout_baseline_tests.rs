//! A line's baseline as Blink places it (#368): the primary font's
//! ascent and descent rounded to device pixels, the half-leading floored
//! to one. Geist at 10 px has ascent 10.05 and descent 2.95: 10 and 3.
//! The expected values at 2x and 2.625x are headless Chromium 155's
//! (an inline-block's bottom on the baseline, 2026-11-15).

use crate::{Family, FontSystem, Layout, LayoutOptions, LineHeight, Style};

/// 10 px-style text: `size` px Geist in a `line` px line.
fn style(size: f32, line: f32) -> Style<'static> {
    let mut style = Style::new(Family::Geist, size);
    style.line_height = LineHeight::Px(line);
    style
}

/// "Row 0" in `style` at `scale`, on the bundle (and `fallback`, if any).
fn laid_out(style: &Style<'_>, scale: f32, system: &mut FontSystem) -> Layout {
    let options = LayoutOptions {
        scale,
        ..LayoutOptions::default()
    };
    Layout::new(system, "Row 0", style, &options)
}

/// The first line's baseline and the first glyph's, device pixels from
/// the layout's top.
fn baselines(layout: &Layout) -> (f32, f32) {
    let line = layout.lines().next().expect("a line").baseline;
    (line, layout.glyphs().next().expect("a glyph").y)
}

#[test]
fn a_negative_half_leading_is_floored_to_a_whole_device_pixel() {
    // Given: (12 - 10 - 3) / 2 = -0.5, floored to -1
    let style = style(10.0, 12.0);

    // When
    let layout = laid_out(&style, 1.0, &mut FontSystem::bundle_only());

    // Then: the baseline at 10 - 1
    assert_eq!(baselines(&layout), (9.0, 9.0));
}

#[test]
fn a_positive_half_leading_is_floored_too() {
    // Given: (16 - 13) / 2 = 1.5, floored to 1
    let style = style(10.0, 16.0);

    // When
    let layout = laid_out(&style, 1.0, &mut FontSystem::bundle_only());

    // Then: the baseline at 10 + 1
    assert_eq!(baselines(&layout), (11.0, 11.0));
}

#[test]
fn at_2x_the_metrics_are_scaled_before_they_are_rounded() {
    // Given: ascent 20.1 and descent 5.9 round to 20 and 6; (24 - 26) / 2 = -1
    let style = style(10.0, 12.0);

    // When
    let layout = laid_out(&style, 2.0, &mut FontSystem::bundle_only());

    // Then
    assert_eq!(baselines(&layout), (19.0, 19.0));
}

/// Chromium's baselines where Blink's Linux descent adjustment would
/// show (a descent that rounds down: 7.08 at 24 device px): it does
/// not, in headless Chromium 155 at 2x and 2.625x.
#[test]
fn baselines_match_chromium_where_the_descent_rounds_down_and_at_a_fractional_scale() {
    // Given: (size, line height, scale) and Chromium's baseline, device px
    let cases = [
        ((12.0, 16.0, 2.0), 24.0),
        ((10.0, 12.0, 2.625), 24.0),
        ((11.0, 14.0, 2.625), 28.0),
    ];

    for ((size, line, scale), chromium) in cases {
        // When
        let layout = laid_out(&style(size, line), scale, &mut FontSystem::bundle_only());

        // Then
        assert_eq!(
            baselines(&layout),
            (chromium, chromium),
            "{size}/{line} at {scale}x"
        );
    }
}

#[test]
fn normal_line_height_puts_the_baseline_at_the_rounded_ascent() {
    // Given: normal is the rounded ascent and descent, 13 px
    let mut style = Style::new(Family::Geist, 10.0);
    style.line_height = LineHeight::Normal;

    // When
    let layout = laid_out(&style, 1.0, &mut FontSystem::bundle_only());

    // Then
    assert_eq!(baselines(&layout).0, 10.0);
}

const CJK_TEST_FONT: &[u8] = include_bytes!("../fonts/test/DroidSansFallback-subset-6F22.ttf");

#[test]
fn a_fallback_font_on_the_line_leaves_the_baseline_on_the_primary_fonts() {
    // Given: a Han fallback on a 13 px line 16 px tall
    let mut system = FontSystem::bundle_only();
    system.register_fallback(CJK_TEST_FONT.to_vec(), *b"Hani");
    let options = LayoutOptions::default();

    // When
    let layout = Layout::new(&mut system, "Row 漢", &style(13.0, 16.0), &options);

    // Then: Blink takes the primary font's metrics for a fixed line
    // height, 13 + floor((16 - 17) / 2) = 12, Chromium's 13/16 baseline
    assert_eq!(layout.lines().next().expect("a line").baseline, 12.0);
    let ys: Vec<f32> = layout.glyphs().map(|glyph| glyph.y).collect();
    assert_eq!(ys, [12.0; 5]);
}
