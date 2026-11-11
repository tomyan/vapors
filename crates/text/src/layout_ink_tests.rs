use vapors_scene::Rect;

use crate::raster::{GlyphRequest, Hinting, Rasterizer, subpixel_of};
use crate::{Family, FontSystem, Layout, LayoutOptions, Style};

/// Each inked glyph's bitmap as the text pipeline places it for a layout
/// at `[x, y]`: whole-pixel x, baseline rounded (`quads.rs`).
fn bitmaps(layout: &Layout, [x, y]: [f32; 2]) -> Vec<Rect> {
    let mut rasterizer = Rasterizer::new();
    let hinting = Hinting::for_scale(layout.scale());
    let mut rects = Vec::new();
    for glyph in layout.glyphs() {
        let (pixel_x, subpixel) = subpixel_of(x + glyph.x);
        let request = GlyphRequest {
            font: glyph.font,
            id: glyph.id,
            size: glyph.size,
            coords: glyph.coords,
            subpixel,
            hinting,
            skew: glyph.skew,
        };
        let coverage = rasterizer.rasterize(&request).expect("draws");
        if !coverage.is_empty() {
            rects.push(bitmap_rect(pixel_x, (y + glyph.y).round(), &coverage));
        }
    }
    rects
}

#[allow(clippy::cast_precision_loss)] // small pixel positions
fn bitmap_rect(x: i32, baseline: f32, coverage: &crate::raster::Coverage) -> Rect {
    let left = (x + coverage.left) as f32;
    let top = baseline + coverage.top as f32;
    Rect::new(
        left,
        top,
        left + coverage.width as f32,
        top + coverage.height as f32,
    )
}

fn layout(text: &str, style: &Style<'_>) -> Layout {
    scaled(&mut FontSystem::bundle_only(), text, style, 1.0)
}

fn scaled(fonts: &mut FontSystem, text: &str, style: &Style<'_>, scale: f32) -> Layout {
    let options = LayoutOptions {
        scale,
        ..LayoutOptions::default()
    };
    Layout::new(fonts, text, style, &options)
}

/// Fails naming the glyph when a bitmap of `laid` at `origin` lies
/// outside its ink moved to `origin`.
fn assert_inked_inside(laid: &Layout, origin: [f32; 2], case: &str) {
    let ink = laid.ink().offset(origin);
    for bitmap in bitmaps(laid, origin) {
        assert!(inside(bitmap, ink), "{case}: {bitmap:?} outside {ink:?}");
    }
}

fn inside(inner: Rect, outer: Rect) -> bool {
    inner.left >= outer.left
        && inner.top >= outer.top
        && inner.right <= outer.right
        && inner.bottom <= outer.bottom
}

#[test]
fn every_glyphs_bitmap_lies_inside_the_layouts_ink() {
    // Given: accents, descenders, stacked marks, synthetic italic, a
    // heavy weight and sizes from small to large
    let base = Style::new(Family::Geist, 16.0);
    let styles = [
        base.clone(),
        Style {
            italic: true,
            ..base.clone()
        },
        Style {
            weight: 900.0,
            size: 40.0,
            ..base.clone()
        },
        Style {
            size: 7.0,
            ..base.clone()
        },
        Style::new(Family::GeistMono, 24.0),
    ];
    let texts = [
        "Héllo Åjgy",
        "f\u{0302}\u{0303}\u{0301}j\u{0323}\u{0324}",
        "QWy_|[]{}",
    ];

    for style in &styles {
        for text in texts {
            // When
            let laid = layout(text, style);

            // Then
            assert_inked_inside(&laid, [0.0, 0.0], &format!("{text:?} {style:?}"));
        }
    }
}

#[test]
fn every_glyphs_bitmap_lies_inside_the_ink_at_scales_1_5_and_2() {
    let styles = [
        Style::new(Family::Geist, 13.0),
        Style {
            italic: true,
            weight: 900.0,
            ..Style::new(Family::Geist, 16.0)
        },
    ];
    for scale in [1.5, 2.0] {
        for style in &styles {
            // When
            let laid = scaled(&mut FontSystem::bundle_only(), "Héllo Åjgy_|", style, scale);

            // Then
            assert_inked_inside(&laid, [0.0, 0.0], &format!("x{scale} {style:?}"));
        }
    }
}

#[test]
fn every_glyphs_bitmap_lies_inside_the_ink_at_fractional_origins() {
    // Given: origins that round the baseline up and down and move the pen
    // across a pixel and a subpixel bin
    let laid = layout("Héllo Åjgy_|", &Style::new(Family::Geist, 16.0));
    for origin in [[0.3, 0.7], [0.5, 0.5], [0.99, 0.01], [-0.4, 12.6]] {
        // Then
        assert_inked_inside(&laid, origin, &format!("at {origin:?}"));
    }
}

/// Set to `1` where a CJK system font is installed (CI installs
/// fonts-noto-cjk): the test then requires the glyph to come from it.
/// Unset, the glyph may be the bundle's .notdef, whose ink is checked
/// instead, since whether this machine has such a font is unknown.
#[cfg(not(target_family = "wasm"))]
#[test]
fn a_system_fallback_glyphs_bitmap_lies_inside_the_ink() {
    // Given
    let required = std::env::var("VAPORS_SYSTEM_FONTS").is_ok_and(|v| v == "1");
    let mut fonts = FontSystem::new();
    let style = Style::new(Family::Geist, 16.0);

    // When
    let laid = scaled(&mut fonts, "漢字", &style, 1.5);

    // Then
    let glyph = laid.glyphs().next().expect("a glyph");
    assert!(
        !required || !fonts.is_bundled(glyph.font),
        "VAPORS_SYSTEM_FONTS=1 but no system font covers U+6F22. \
         Fix: install a CJK font (apt-get install fonts-noto-cjk) or unset VAPORS_SYSTEM_FONTS"
    );
    assert_inked_inside(&laid, [0.25, 0.75], "漢字");
}

#[test]
fn an_empty_layout_inks_nothing() {
    // When
    let laid = layout("", &Style::new(Family::Geist, 16.0));

    // Then
    assert_eq!(laid.ink(), Rect::default());
}
