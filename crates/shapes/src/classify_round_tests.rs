//! Rounded rectangles: SVG's `<rect rx ry>` outline, from any of its
//! points and either way round, is a rect with radii; a near miss is a
//! path.

use crate::canonical::{FillRule, Paint};
use crate::classify::{Classified, classify};
use crate::path::IconPath;

const FILL: Paint = Paint::Fill(FillRule::NonZero);
const STROKE: Paint = Paint::Stroke { width: 1.5 };

fn classified(data: &str, paint: Paint) -> Classified {
    let path = IconPath::parse(data).unwrap_or_else(|e| panic!("{data:?}: {e}"));
    classify(&path, paint)
}

const fn rounded(radius: [f32; 2]) -> Classified {
    Classified::Rect {
        origin: [1.0, 1.0],
        size: [10.0, 6.0],
        radius,
    }
}

#[test]
fn svgs_rounded_rect_outline_is_a_rect_with_its_radii() {
    // Given: SVG 2's outline for x=1 y=1 width=10 height=6 rx=ry=2,
    // filled and stroked; open, filled; from an arc's start; the other
    // way round; elliptical corners, written relative
    let round = rounded([2.0, 2.0]);
    let spec = "M3 1H9A2 2 0 0 1 11 3V5A2 2 0 0 1 9 7H3A2 2 0 0 1 1 5V3A2 2 0 0 1 3 1Z";
    let cases = [
        (spec, STROKE, round),
        (spec, FILL, round),
        (spec.trim_end_matches('Z'), FILL, round),
        (
            "M1 3A2 2 0 0 1 3 1H9A2 2 0 0 1 11 3V5A2 2 0 0 1 9 7H3A2 2 0 0 1 1 5V3Z",
            STROKE,
            round,
        ),
        (
            "M3 1A2 2 0 0 0 1 3V5A2 2 0 0 0 3 7H9A2 2 0 0 0 11 5V3A2 2 0 0 0 9 1H3Z",
            STROKE,
            round,
        ),
        (
            "M4 1h4a3 1.5 0 0 1 3 1.5v3a3 1.5 0 0 1-3 1.5h-4a3 1.5 0 0 1-3-1.5v-3a3 1.5 0 0 1 3-1.5z",
            STROKE,
            rounded([3.0, 1.5]),
        ),
    ];

    // When, Then
    for (data, paint, expected) in cases {
        assert_eq!(classified(data, paint), expected, "{data}");
    }
}

/// Asserts each of `cases` stays a path.
fn paths(cases: &[(&str, Paint)]) {
    for &(data, paint) in cases {
        assert_eq!(classified(data, paint), Classified::Path, "{data}");
    }
}

#[test]
fn a_rounded_rect_with_a_segment_out_of_place_is_a_path() {
    // Given: an arc 0.001 short; one corner of radius 1; an extra
    // segment (a side in two lines); an open stroke; a second subpath;
    // a top side sloping 0.2, with its right side short to match
    let rest = "A2 2 0 0 1 9 7H3A2 2 0 0 1 1 5V3A2 2 0 0 1 3 1";
    paths(&[
        (&format!("M3 1H9A2 2 0 0 1 10.999 3V5{rest}Z"), FILL),
        (&format!("M3 1H10A1 1 0 0 1 11 2V5{rest}Z"), FILL),
        (&format!("M3 1H6H9A2 2 0 0 1 11 3V5{rest}Z"), FILL),
        (&format!("M3 1H9A2 2 0 0 1 11 3V5{rest}"), STROKE),
        (&format!("M3 1H9A2 2 0 0 1 11 3V5{rest}ZM1 1h1v1z"), FILL),
        (&format!("M3 1L9 1.2A2 2 0 0 1 11 3.2V5{rest}Z"), FILL),
    ]);
}

#[test]
fn a_rounded_rect_with_an_arc_of_another_shape_is_a_path() {
    // Given: one corner turned in; every corner turned in; a rotated
    // arc; a large arc; two arcs spanning other than their radii (the
    // top side shifted 0.5 right); one arc of radius 3 spanning 2 x 2;
    // corners below the closed form's floor (geometry::MIN_RADIUS)
    paths(&[
        (
            "M3 1H9A2 2 0 0 0 11 3V5A2 2 0 0 1 9 7H3A2 2 0 0 1 1 5V3A2 2 0 0 1 3 1Z",
            FILL,
        ),
        (
            "M3 1H9A2 2 0 0 0 11 3V5A2 2 0 0 0 9 7H3A2 2 0 0 0 1 5V3A2 2 0 0 0 3 1Z",
            FILL,
        ),
        (
            "M3 1H9A2 2 30 0 1 11 3V5A2 2 0 0 1 9 7H3A2 2 0 0 1 1 5V3A2 2 0 0 1 3 1Z",
            FILL,
        ),
        (
            "M3 1H9A2 2 0 1 1 11 3V5A2 2 0 0 1 9 7H3A2 2 0 0 1 1 5V3A2 2 0 0 1 3 1Z",
            FILL,
        ),
        (
            "M2.5 1H8.5A2 2 0 0 1 11 3V5A2 2 0 0 1 9 7H3A2 2 0 0 1 1 5V3A2 2 0 0 1 2.5 1Z",
            FILL,
        ),
        (
            "M3 1H9A2 2 0 0 1 11 3V5A2 2 0 0 1 9 7H3A3 3 0 0 1 1 5V3A2 2 0 0 1 3 1Z",
            FILL,
        ),
        (
            "M1e-10 0H3e-10A1e-10 1e-10 0 0 1 4e-10 1e-10V3e-10A1e-10 1e-10 0 0 1 3e-10 4e-10H1e-10A1e-10 1e-10 0 0 1 0 3e-10V1e-10A1e-10 1e-10 0 0 1 1e-10 0Z",
            FILL,
        ),
    ]);
}
