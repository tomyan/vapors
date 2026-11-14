//! The segments the pen builds: arcs kept and corrected, the current
//! point exact after an arc and a close, negative zero as zero.

use zeno::{Command, Point};

use super::*;

fn at(x: f32, y: f32) -> Point {
    Point::new(x, y)
}

#[test]
fn segments_keep_arcs_and_track_the_pen_through_a_close() {
    // When
    let path = IconPath::parse("M1 1h2a3 4 30 1 0 2 2zl1 1").expect("parses");

    // Then: the arc stays an arc; after Z the pen is back at the start
    assert_eq!(
        path.segments(),
        [
            Segment::Move(at(1.0, 1.0)),
            Segment::Line(at(3.0, 1.0)),
            Segment::Arc {
                radii: [3.0, 4.0],
                rotation: 30.0,
                large: true,
                sweep: false,
                to: at(5.0, 3.0),
            },
            Segment::Close,
            Segment::Line(at(2.0, 2.0)),
        ]
    );
    // and zeno draws the arc as curves, one or more
    assert!(
        path.commands().len() > path.segments().len(),
        "{:?}",
        path.commands()
    );
}

#[test]
fn a_relative_command_after_an_arc_starts_from_the_exact_end_point() {
    // When: zeno's curves end within rounding of the arc's end; the pen
    // is at the end point written
    let path = IconPath::parse("M0 0a4 4 30 0 1 4 4h1").expect("parses");

    // Then
    assert_eq!(path.commands().last(), Some(&Command::LineTo(at(5.0, 4.0))));
    assert_eq!(path.segments().last(), Some(&Segment::Line(at(5.0, 4.0))));
}

#[test]
fn negative_zero_is_zero() {
    // When: a rotation of -0, which no addition would normalise
    let path = IconPath::parse("M0 0A4 4 -0 0 1 4 4").expect("parses");

    // Then: its bits are zero's, so one shape has one canonical form
    let Segment::Arc { rotation, .. } = path.segments()[1] else {
        panic!("an arc")
    };
    assert_eq!(rotation.to_bits(), 0, "{rotation}");
}
