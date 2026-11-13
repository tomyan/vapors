//! Cubic and quadratic curves: control points absolute and relative,
//! and repeated.

use zeno::{Command, Point};

use super::*;

fn parsed(data: &str) -> Vec<Command> {
    IconPath::parse(data)
        .unwrap_or_else(|error| panic!("{data:?}: {error}"))
        .commands()
        .to_vec()
}

fn at(x: f32, y: f32) -> Point {
    Point::new(x, y)
}

#[test]
fn cubic_and_quadratic_curves_parse_with_their_control_points() {
    // When
    let commands = parsed("M1 1C2 0 3 0 4 1Q5 2 6 1");

    // Then
    assert_eq!(
        commands,
        [
            Command::MoveTo(at(1.0, 1.0)),
            Command::CurveTo(at(2.0, 0.0), at(3.0, 0.0), at(4.0, 1.0)),
            Command::QuadTo(at(5.0, 2.0), at(6.0, 1.0)),
        ]
    );
}

#[test]
fn relative_curves_take_every_point_from_the_current_point() {
    // When
    let commands = parsed("M10 10c1 -1 2 -1 3 0q1 1 2 0");

    // Then
    assert_eq!(
        commands,
        [
            Command::MoveTo(at(10.0, 10.0)),
            Command::CurveTo(at(11.0, 9.0), at(12.0, 9.0), at(13.0, 10.0)),
            Command::QuadTo(at(14.0, 11.0), at(15.0, 10.0)),
        ]
    );
}

#[test]
fn repeated_curve_coordinates_repeat_the_curve() {
    // When
    let commands = parsed("M0 0q1 1 2 0 1 -1 2 0");

    // Then
    assert_eq!(
        commands,
        [
            Command::MoveTo(at(0.0, 0.0)),
            Command::QuadTo(at(1.0, 1.0), at(2.0, 0.0)),
            Command::QuadTo(at(3.0, -1.0), at(4.0, 0.0)),
        ]
    );
}
