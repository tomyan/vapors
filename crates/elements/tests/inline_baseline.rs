//! An inline element's box sits on the line's baseline as the glyphs do
//! (#368): Geist at 10 px in a 12 px line has its baseline 9 px below
//! the line's top (ascent 10, descent 3, the half-leading -0.5 floored
//! to -1, as Chromium places it), so a span's box, its content area 10
//! above and 3 below the baseline, starts 1 px above the line. Headless
//! Chromium 155 gives the span top -1 and height 13 at 1x, -0.5 and 13
//! at 2x (2026-11-15).

mod support;

use support::{document, element, text, viewport};

#[test]
fn a_spans_box_starts_at_the_baseline_less_its_ascent_with_the_half_leading_floored() {
    for (scale, wanted) in [(1.0_f32, (-1.0, 13.0)), (2.0, (-0.5, 13.0))] {
        // Given
        let mut doc = document();
        let body = doc.body();
        let p = element(&mut doc, body, "p", "");
        let span = element(&mut doc, p, "span", "mark");
        text(&mut doc, span, "Row 0");
        doc.add_stylesheet(
            "app.css",
            "body { margin: 0; } p { margin: 0; font-size: 10px; line-height: 12px; } \
             .mark { background-color: #ffcc00; }",
        )
        .expect("parses");

        // When
        let mut at = viewport(200.0, 100.0);
        at.scale = scale;
        doc.update(at).expect("updates");

        // Then: at 2x, ascent 20.1 and descent 5.9 round to 20 and 6, and
        // (24 - 26) / 2 = -1 device px: the baseline 19 device px down
        let rect = doc
            .painted()
            .and_then(|p| p.border_box(span))
            .expect("painted");
        assert_eq!((rect.y, rect.height), wanted, "at {scale}x");
    }
}
