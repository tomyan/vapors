//! RELAXED (lead's decision, 2026-11-10): at two pixels of touche's Mute
//! icon at 1x, the paint rules compare vapors with exact coverage, not
//! with Chromium. Mute's X (`M11 6l3 4M14 6l-3 4`, stroke 1.5) crosses
//! at y = 8; the pixels are svg (11, 8) and (13, 8), in each of the
//! three rows of `icons_paint`. Exact coverage there, from 32x32
//! supersampling of the union of the two strokes (computed below from
//! the path), is 0.789: 73 on the dark row's #ececf0 button, where
//! rows 7 and 8 read `231 73 29 73 231`. vapors draws `231 45 29 47 232`
//! (docs/icons.md: zeno sums overlapping coverage). Chromium 155 draws
//! 206 at row 8 (`236 206 45 206 236`), off exact by 133, though the
//! geometry is symmetric about y = 8 and Chromium draws row 7 at 63.
//! The same `EDGE_BOUND` applies to the exact value as to Chromium's.
//! Every other pixel, Mute's included, is compared with Chromium.
//! Retire this when Chromium is within `EDGE_BOUND` of vapors at these
//! pixels.

use vapors_text::comparison::Rgba;

/// Mute's crossing strokes, as in `fixtures/icons_paint.wisp`.
const CROSS_PATH: &str = "M11 6l3 4M14 6l-3 4";
const STROKE_WIDTH: f32 = 1.5;
const SAMPLES: u32 = 32;
/// The pixels, in the svg's own 16 px at 1x.
const PIXELS: [(u32, u32); 2] = [(11, 8), (13, 8)];
/// Mute's svg origin in each row at 1x: its button at 16 + 8 * 52,
/// the svg 14 px into it.
const SVG_LEFT: u32 = 446;
/// Each row's svg top, its button colour, and its stroke colour with
/// alpha (`icons_paint.wisp`).
const ROWS: [(u32, [f32; 3], [f32; 4]); 3] = [
    (30, [236.0, 236.0, 240.0], [29.0, 29.0, 31.0, 1.0]),
    (90, [29.0, 29.0, 31.0], [245.0, 245.0, 247.0, 1.0]),
    (150, [236.0, 236.0, 240.0], [29.0, 29.0, 31.0, 0.5]),
];

type Segment = ((f32, f32), (f32, f32));

/// Chromium's rendering with the declared pixels replaced by exact
/// coverage, for `icons_paint` at 1x; unchanged otherwise.
pub fn with_exact_pixels(name: &str, scale: u32, mut theirs: Rgba) -> Rgba {
    if name != "icons_paint" || scale != 1 {
        return theirs;
    }
    let segments = segments(CROSS_PATH);
    for (top, button, stroke) in ROWS {
        for (x, y) in PIXELS {
            let coverage = coverage(&segments, x, y);
            let i = (((top + y) * theirs.width + SVG_LEFT + x) * 4) as usize;
            for c in 0..3 {
                theirs.data[i + c] = blend(button[c], stroke[c], stroke[3] * coverage);
            }
        }
    }
    theirs
}

/// Segments of a path of `M x y` and `l dx dy` only; anything else panics.
fn segments(path: &str) -> Vec<Segment> {
    let mut out = Vec::new();
    let mut at = (0.0, 0.0);
    let mut rest = path;
    while let Some(op) = rest.chars().next() {
        let end = rest[1..]
            .find(|c: char| c.is_ascii_alphabetic())
            .map_or(rest.len(), |n| n + 1);
        let numbers: Vec<f32> = rest[1..end]
            .split_whitespace()
            .map(|n| n.parse().expect("a number in the Mute path"))
            .collect();
        assert!(numbers.len() == 2, "{op} takes two numbers in {path}");
        match op {
            'M' => at = (numbers[0], numbers[1]),
            'l' => {
                let to = (at.0 + numbers[0], at.1 + numbers[1]);
                out.push((at, to));
                at = to;
            }
            _ => panic!("unsupported command {op} in {path}"),
        }
        rest = &rest[end..];
    }
    out
}

/// Fraction of the pixel's sample points inside any stroked segment.
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
fn coverage(segments: &[Segment], x: u32, y: u32) -> f32 {
    let step = 1.0 / SAMPLES as f32;
    let mut inside = 0;
    for i in 0..SAMPLES {
        for j in 0..SAMPLES {
            let p = (
                x as f32 + (i as f32 + 0.5) * step,
                y as f32 + (j as f32 + 0.5) * step,
            );
            inside += u32::from(segments.iter().any(|s| stroked(*s, p)));
        }
    }
    inside as f32 / (SAMPLES * SAMPLES) as f32
}

/// Whether `p` is inside the segment's stroke, butt caps.
fn stroked(((x0, y0), (x1, y1)): Segment, (px, py): (f32, f32)) -> bool {
    let (dx, dy) = (x1 - x0, y1 - y0);
    let length = dx.hypot(dy);
    let along = ((px - x0) * dx + (py - y0) * dy) / length;
    let across = ((px - x0) * dy - (py - y0) * dx) / length;
    (0.0..=length).contains(&along) && across.abs() <= STROKE_WIDTH / 2.0
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn blend(under: f32, over: f32, amount: f32) -> u8 {
    (under + (over - under) * amount).round() as u8
}

#[test]
fn mute_crossing_coverage_is_computed_from_the_fixtures_path() {
    // Given
    let fixture = include_str!("../../fixtures/icons_paint.wisp");
    let segments = segments(CROSS_PATH);

    // When
    let found = coverage(&segments, 11, 8);

    // Then
    assert!(fixture.contains(CROSS_PATH), "fixture lost {CROSS_PATH}");
    assert!((found - 0.789).abs() < 0.01, "coverage {found}");
}
