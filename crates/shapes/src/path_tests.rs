//! The subset of SVG path data: what parses to which commands.
//! Arcs are in `path_arc_tests.rs`, curves in `path_curve_tests.rs`,
//! the pen's segments in `path_pen_tests.rs`, refusals in
//! `path_refusals_tests.rs`.

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
fn absolute_lines_and_close_parse_in_order() {
    // When
    let commands = parsed("M1.5 4.5L14.5 4.5Z");

    // Then
    assert_eq!(
        commands,
        [
            Command::MoveTo(at(1.5, 4.5)),
            Command::LineTo(at(14.5, 4.5)),
            Command::Close
        ]
    );
}

#[test]
fn relative_and_axis_lines_move_from_the_current_point() {
    // When
    let commands = parsed("M1.5 4.5v8h13l-1.5-1.5");

    // Then
    assert_eq!(
        commands,
        [
            Command::MoveTo(at(1.5, 4.5)),
            Command::LineTo(at(1.5, 12.5)),
            Command::LineTo(at(14.5, 12.5)),
            Command::LineTo(at(13.0, 11.0)),
        ]
    );
}

#[test]
fn absolute_axis_lines_keep_the_other_coordinate() {
    // When
    let commands = parsed("M5 3H3V14");

    // Then
    assert_eq!(
        commands,
        [
            Command::MoveTo(at(5.0, 3.0)),
            Command::LineTo(at(3.0, 3.0)),
            Command::LineTo(at(3.0, 14.0)),
        ]
    );
}

#[test]
fn repeated_coordinates_repeat_the_command() {
    // When
    let commands = parsed("M9 4l4 4-4 4");

    // Then
    assert_eq!(
        commands[1..],
        [
            Command::LineTo(at(13.0, 8.0)),
            Command::LineTo(at(9.0, 12.0))
        ]
    );
}

#[test]
fn coordinates_after_a_move_are_lines() {
    // When
    let commands = parsed("m1 1 2 2M5 5 6 6");

    // Then
    assert_eq!(
        commands,
        [
            Command::MoveTo(at(1.0, 1.0)),
            Command::LineTo(at(3.0, 3.0)),
            Command::MoveTo(at(5.0, 5.0)),
            Command::LineTo(at(6.0, 6.0)),
        ]
    );
}

#[test]
fn a_relative_move_after_close_starts_from_the_subpath_start() {
    // When
    let commands = parsed("M2 2h4zm1 1h1");

    // Then
    assert_eq!(commands[3], Command::MoveTo(at(3.0, 3.0)));
}

#[test]
fn packed_numbers_signs_and_exponents_split_as_svg_reads_them() {
    // When
    let commands = parsed("M1.5.5L-1e1,2E-1");

    // Then
    assert_eq!(
        commands,
        [
            Command::MoveTo(at(1.5, 0.5)),
            Command::LineTo(at(-10.0, 0.2))
        ]
    );
}

#[test]
fn an_arc_ends_at_its_end_point_through_curves() {
    // When
    let commands = parsed("M2 6h7a4 4 0 0 1 0 8H6");

    // Then
    let arc = &commands[2..commands.len() - 1];
    assert!(!arc.is_empty(), "{commands:?}");
    assert!(
        arc.iter().all(|c| matches!(c, Command::CurveTo(..))),
        "{arc:?}"
    );
    let Some(Command::CurveTo(_, _, end)) = arc.last() else {
        panic!("{arc:?}")
    };
    assert!(
        (end.x - 9.0).abs() < 1e-4 && (end.y - 14.0).abs() < 1e-4,
        "{end:?}"
    );
}

#[test]
fn the_sweep_flag_chooses_the_side_the_arc_bulges() {
    // Given: a half circle from (2, 6) to (2, 14), sweep 1 bulging right
    // (clockwise on screen, y down) and sweep 0 bulging left.
    let rightmost = |data: &str| {
        parsed(data)
            .iter()
            .filter_map(|c| match c {
                Command::CurveTo(a, b, _) => Some(a.x.max(b.x)),
                _ => None,
            })
            .fold(f32::MIN, f32::max)
    };

    // When
    let (clockwise, anticlockwise) = (
        rightmost("M2 6A4 4 0 0 1 2 14"),
        rightmost("M2 6A4 4 0 0 0 2 14"),
    );

    // Then
    assert!(
        clockwise > 5.0 && anticlockwise < 2.5,
        "{clockwise} {anticlockwise}"
    );
}

#[test]
fn packed_arc_flags_parse() {
    // When
    let spaced = parsed("M4 8a4 4 0 0 0 8 0");
    let packed = parsed("M4 8a4 4 0 008 0");

    // Then
    assert_eq!(spaced, packed);
}
