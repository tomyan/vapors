//! Arcs in the icon subset: the edge cases SVG defines and zeno's
//! arc conversion must see handled.

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

/// The largest x among the points the arc's curves are built from.
fn rightmost(data: &str) -> f32 {
    parsed(data)
        .iter()
        .filter_map(|command| match command {
            Command::CurveTo(a, b, end) => Some(a.x.max(b.x).max(end.x)),
            _ => None,
        })
        .fold(f32::MIN, f32::max)
}

#[test]
fn an_arc_with_a_zero_radius_is_a_straight_line() {
    // When
    let zero_x = parsed("M1 1A0 4 0 0 1 5 5");
    let zero_y = parsed("M1 1a4 0 0 0 1 3 2");

    // Then
    assert_eq!(
        zero_x,
        [Command::MoveTo(at(1.0, 1.0)), Command::LineTo(at(5.0, 5.0))]
    );
    assert_eq!(
        zero_y,
        [Command::MoveTo(at(1.0, 1.0)), Command::LineTo(at(4.0, 3.0))]
    );
}

#[test]
fn negative_radii_are_taken_as_absolute() {
    // When
    let negative = parsed("M2 6A-4 -4 0 0 1 2 14");

    // Then
    assert_eq!(negative, parsed("M2 6A4 4 0 0 1 2 14"));
}

#[test]
fn radii_too_small_for_the_chord_are_scaled_up() {
    // When
    let scaled = parsed("M2 6A1 1 0 0 1 2 14");

    // Then
    assert_eq!(scaled, parsed("M2 6A4 4 0 0 1 2 14"));
    assert!(rightmost("M2 6A1 1 0 0 1 2 14") > 5.0);
}

#[test]
fn an_arc_to_the_current_point_draws_nothing() {
    // When
    let commands = parsed("M3 3A4 4 0 0 1 3 3L5 5");

    // Then
    assert_eq!(
        commands,
        [Command::MoveTo(at(3.0, 3.0)), Command::LineTo(at(5.0, 5.0))]
    );
}

#[test]
fn the_large_arc_flag_chooses_the_longer_arc_of_a_short_chord() {
    // Given: a chord of 4 inside circles of radius 4, so the circle's
    // centre is 3.46 to one side of the chord and the arcs differ.

    // When
    let (small, large) = (
        rightmost("M2 8A4 4 0 0 1 2 12"),
        rightmost("M2 8A4 4 0 1 1 2 12"),
    );

    // Then: the small arc bulges 0.54 past the chord, the large one 7.46.
    assert!(small < 3.0 && large > 9.0, "{small} {large}");
}

/// The segments, which carry the corrections the commands draw.
fn segments(data: &str) -> Vec<Segment> {
    IconPath::parse(data)
        .unwrap_or_else(|error| panic!("{data:?}: {error}"))
        .segments()
        .to_vec()
}

#[test]
fn the_segments_carry_the_arc_corrections_the_commands_draw() {
    // When, Then: a zero radius is a line; negative radii absolute;
    // radii too small for the chord scaled up (a chord of 8 needs 4);
    // an arc to the current point is dropped
    assert_eq!(
        segments("M1 1A0 4 0 0 1 5 5"),
        [Segment::Move(at(1.0, 1.0)), Segment::Line(at(5.0, 5.0))]
    );
    let corrected = Segment::Arc {
        radii: [4.0, 4.0],
        rotation: 0.0,
        large: false,
        sweep: true,
        to: at(2.0, 14.0),
    };
    assert_eq!(segments("M2 6A-4 -4 0 0 1 2 14")[1], corrected);
    assert_eq!(segments("M2 6A1 1 0 0 1 2 14")[1], corrected);
    assert_eq!(segments("M2 6A4 4 0 0 1 2 14")[1], corrected);
    assert_eq!(
        segments("M3 3A4 4 0 0 1 3 3L5 5"),
        [Segment::Move(at(3.0, 3.0)), Segment::Line(at(5.0, 5.0))]
    );
}

#[test]
fn radii_too_small_under_a_rotation_are_scaled_by_the_rotated_chord() {
    // Given: a chord of (4, 4) under a 45 degree rotation lies along
    // the ellipse's x axis, so only rx must reach half its length, 2.83
    let Segment::Arc { radii, .. } = segments("M0 0A1 1 45 0 1 4 4")[1] else {
        panic!("an arc")
    };

    // Then: lambda = 8 / 1, both radii scaled by sqrt(8)
    assert!((radii[0] - 2.828_427).abs() < 1e-5, "{radii:?}");
    assert!((radii[1] - 2.828_427).abs() < 1e-5, "{radii:?}");
}
