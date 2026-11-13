//! A rect path classified as a rect, two arcs as a circle, other paths
//! as meshes; a degenerate closed form refused naming the shape.

use super::*;
use crate::canonical::FillRule;

const FILL: Paint = Paint::Fill(FillRule::NonZero);
const STROKE: Paint = Paint::Stroke { width: 1.5 };

fn classified(data: &str, paint: Paint) -> Classified {
    let path = IconPath::parse(data).unwrap_or_else(|e| panic!("{data:?}: {e}"));
    classify("a.wisp:3", &path, paint).unwrap_or_else(|e| panic!("{data:?}: {e}"))
}

fn refused(data: &str, paint: Paint) -> String {
    let path = IconPath::parse(data).unwrap_or_else(|e| panic!("{data:?}: {e}"));
    classify("b.wisp:7", &path, paint).expect_err("refused")
}

const RECT: Classified = Classified::Rect {
    origin: [1.0, 1.0],
    size: [4.0, 3.0],
};

#[test]
fn a_closed_axis_aligned_rectangle_is_a_rect() {
    // Given: written four ways, from any corner, either way round
    let cases = [
        ("M1 1h4v3h-4z", STROKE),
        ("M1 1h4v3h-4z", FILL),
        ("M1 1L5 1L5 4L1 4", FILL),
        ("M1 1L5 1L5 4L1 4L1 1", FILL),
        ("M1 1L5 1L5 4L1 4L1 1Z", FILL),
        ("M5 4V1H1V4Z", STROKE),
        ("M1 4v-3h4v3z", STROKE),
    ];

    // When, Then
    for (data, paint) in cases {
        assert_eq!(classified(data, paint), RECT, "{data}");
    }
}

#[test]
fn a_rectangle_a_closed_form_would_draw_differently_is_a_path() {
    // Given: a stroke whose line back to the start adds a join; an open
    // stroke; a rectangle not axis-aligned; five sides; a triangle
    let cases = [
        ("M1 1L5 1L5 4L1 4L1 1Z", STROKE),
        ("M1 1L5 1L5 4L1 4L1 1", STROKE),
        ("M1 1h4v3h-4", STROKE),
        ("M1 1L5 2L5 4L1 4Z", FILL),
        ("M1 1h4v3h-2h-2z", FILL),
        ("M1 1L5 1L3 4Z", FILL),
    ];

    // When, Then
    for (data, paint) in cases {
        assert_eq!(classified(data, paint), Classified::Path, "{data}");
    }
}

#[test]
fn a_rectangle_without_width_or_height_is_refused_naming_the_shape() {
    // When
    let error = refused("M1 1h4v0h-4z", FILL);

    // Then
    assert_eq!(
        error,
        "the shape at b.wisp:7 is a rectangle of size [4.0, 0.0]; a rectangle needs width and height above zero"
    );
}

#[test]
fn two_half_arcs_are_a_circle_or_an_ellipse() {
    // Given: a circle on a horizontal diameter, closed and not, either
    // sweep; an ellipse on a vertical diameter
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
}

#[test]
fn arcs_that_do_not_form_one_ellipse_are_a_path() {
    // Given: radii that differ between the arcs (both fitting the chord,
    // so the pen leaves them); a rotation; sweeps that differ; a chord
    // that is not a diameter; three arcs; not back to the start
    let cases = [
        "M2 8a6 6 0 1 0 12 0a7 7 0 1 0-12 0z",
        "M2 8a6 4 30 1 0 12 0a6 4 30 1 0-12 0z",
        "M2 8a6 6 0 1 0 12 0a6 6 0 1 1-12 0z",
        "M2 8a6 6 0 1 0 10 0a6 6 0 1 0-10 0z",
        "M2 8a6 6 0 0 0 6-6a6 6 0 0 0 6 6a6 6 0 0 0-12 0z",
        "M2 8a6 6 0 1 0 12 0a6 6 0 1 0-12 1z",
    ];

    // When, Then
    for data in cases {
        assert_eq!(classified(data, FILL), Classified::Path, "{data}");
    }
}

#[test]
fn arcs_without_radius_are_lines_and_so_a_path() {
    // When: the pen made each zero-radius arc a line (SVG 2, 9.5.1)
    let classified = classified("M2 8a0 0 0 1 0 12 0a0 0 0 1 0-12 0z", STROKE);

    // Then
    assert_eq!(classified, Classified::Path);
}
