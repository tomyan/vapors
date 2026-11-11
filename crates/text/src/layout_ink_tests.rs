use vapors_scene::Rect;

use crate::raster::{GlyphRequest, Hinting, Rasterizer, subpixel_of};
use crate::{Family, FontSystem, Layout, LayoutOptions, Style};

/// Each inked glyph's bitmap as the text pipeline places it for a layout
/// at the origin: whole-pixel x, baseline rounded (`quads.rs`).
fn bitmaps(layout: &Layout) -> Vec<Rect> {
    let mut rasterizer = Rasterizer::new();
    let hinting = Hinting::for_scale(layout.scale());
    let mut rects = Vec::new();
    for glyph in layout.glyphs() {
        let (pixel_x, subpixel) = subpixel_of(glyph.x);
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
            rects.push(bitmap_rect(pixel_x, glyph.y.round(), &coverage));
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
    Layout::new(
        &mut FontSystem::bundle_only(),
        text,
        style,
        &LayoutOptions::default(),
    )
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
            let ink = laid.ink();
            for bitmap in bitmaps(&laid) {
                assert!(
                    inside(bitmap, ink),
                    "{text:?} {style:?}: {bitmap:?} outside {ink:?}"
                );
            }
        }
    }
}

#[test]
fn an_empty_layout_inks_nothing() {
    // When
    let laid = layout("", &Style::new(Family::Geist, 16.0));

    // Then
    assert_eq!(laid.ink(), Rect::default());
}
