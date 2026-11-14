//! A rect path classified as a rect, two arcs as a circle, other paths
//! as meshes; the forms the closed form would draw differently stay
//! paths, including a rectangle without width or height.

use super::*;
use crate::canonical::FillRule;

const FILL: Paint = Paint::Fill(FillRule::NonZero);
const STROKE: Paint = Paint::Stroke { width: 1.5 };

fn classified(data: &str, paint: Paint) -> Classified {
    let path = IconPath::parse(data).unwrap_or_else(|e| panic!("{data:?}: {e}"));
    classify(&path, paint)
}

const RECT: Classified = Classified::Rect {
    origin: [1.0, 1.0],
    size: [4.0, 3.0],
};

#[test]
fn a_closed_axis_aligned_rectangle_is_a_rect() {
    // Given: written four ways, from any corner, either way round, and
    // a side off by less than the tolerance
    let cases = [
        ("M1 1h4v3h-4z", STROKE),
        ("M1 1h4v3h-4z", FILL),
        ("M1 1L5 1L5 4L1 4", FILL),
        ("M1 1L5 1L5 4L1 4L1 1", FILL),
        ("M1 1L5 1L5 4L1 4L1 1Z", FILL),
        ("M5 4V1H1V4Z", STROKE),
        ("M1 4v-3h4v3z", STROKE),
        ("M1 1L5 1.00003L5 4L1 4z", FILL),
    ];

    // When, Then
    for (data, paint) in cases {
        let found = classified(data, paint);
        assert!(
            matches!(found, Classified::Rect { .. }),
            "{data}: {found:?}"
        );
    }
    assert_eq!(classified("M1 1h4v3h-4z", STROKE), RECT);
}

#[test]
fn a_rectangle_a_closed_form_would_draw_differently_is_a_path() {
    // Given: a stroke whose line back to the start adds a join; an open
    // stroke; a rectangle not axis-aligned; a side off by more than the
    // tolerance; five sides; a triangle; a second subpath after it; a
    // rectangle without height, written either side first (SVG strokes
    // a line, which the tessellator draws)
    let cases = [
        ("M1 1L5 1L5 4L1 4L1 1Z", STROKE),
        ("M1 1L5 1L5 4L1 4L1 1", STROKE),
        ("M1 1h4v3h-4", STROKE),
        ("M1 1L5 2L5 4L1 4Z", FILL),
        ("M1 1L5 1.001L5 4L1 4z", FILL),
        ("M1 1h4v3h-2h-2z", FILL),
        ("M1 1L5 1L3 4Z", FILL),
        ("M1 1h4v3h-4zM6 6h1v1z", FILL),
        ("M1 1h4v3h-4zM6 6h1v1z", STROKE),
        ("M1 1h4v0h-4z", FILL),
        ("M1 1v0h4v0z", FILL),
        ("M1 1h0v4h0z", STROKE),
    ];

    // When, Then
    for (data, paint) in cases {
        assert_eq!(classified(data, paint), Classified::Path, "{data}");
    }
}

#[test]
fn two_half_arcs_are_a_circle_or_an_ellipse() {
    // Given: a circle on a horizontal diameter, closed and not, either
    // sweep; an ellipse on a vertical diameter; a circle far from the
    // origin written with relative arcs
    let circle = Classified::Ellipse {
        centre: [8.0, 8.0],
        radii: [6.0, 6.0],
    };
    let cases = [
        ("M2 8a6 6 0 1 0 12 0a6 6 0 1 0-12 0z", STROKE, circle),
        ("M2 8A6 6 0 1 0 14 8A6 6 0 1 0 2 8", FILL, circle),
        ("M14 8a6 6 0 0 1-12 0a6 6 0 0 1 12 0z", FILL, circle),
        (
            "M8 2a6 4 0 1 0 0 8a6 4 0 1 0 0-8z",
            STROKE,
            Classified::Ellipse {
                centre: [8.0, 6.0],
                radii: [6.0, 4.0],
            },
        ),
    ];

    // When, Then
    for (data, paint, expected) in cases {
        assert_eq!(classified(data, paint), expected, "{data}");
    }
    // and a circle far from the origin whose relative arcs miss the
    // start by an f32 ulp (6e-5 across 1024): within the tolerance,
    // which scales by the coordinates' magnitude, not the radius
    let far = classified(
        "M1023.9999 8a0.1 0.1 0 1 0 0.2 0a0.1 0.1 0 1 0-0.2 0z",
        FILL,
    );
    assert!(
        matches!(
            far,
            Classified::Ellipse {
                radii: [0.1, 0.1],
                ..
            }
        ),
        "{far:?}"
    );
}

#[test]
fn arcs_that_do_not_form_one_ellipse_are_a_path() {
    // Given: radii that differ between the arcs (both fitting the chord,
    // so the pen leaves them); a rotation (a circle is rotation
    // invariant, but the form is not exact, so this is the conservative
    // answer, pinned); sweeps that differ; a chord that is not a
    // diameter; a chord 6e-5 short of the diameter, whose two centres
    // are 0.019 apart; three arcs; not back to the start; zero-radius
    // arcs, which the pen made lines
    let cases = [
        "M2 8a6 6 0 1 0 12 0a7 7 0 1 0-12 0z",
        "M2 8a6 6 30 1 0 12 0a6 6 30 1 0-12 0z",
        "M2 8a6 6 0 1 0 12 0a6 6 0 1 1-12 0z",
        "M2 8a6 6 0 1 0 10 0a6 6 0 1 0-10 0z",
        "M2 8A6 6 0 1 0 13.99994 8A6 6 0 1 0 2 8z",
        "M2 8a6 6 0 0 0 6-6a6 6 0 0 0 6 6a6 6 0 0 0-12 0z",
        "M2 8a6 6 0 1 0 12 0a6 6 0 1 0-12 1z",
        "M2 8a0 0 0 1 0 12 0a0 0 0 1 0-12 0z",
    ];

    // When, Then
    for data in cases {
        assert_eq!(classified(data, FILL), Classified::Path, "{data}");
    }
}
