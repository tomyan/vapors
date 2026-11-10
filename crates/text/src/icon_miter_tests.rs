use super::*;
use crate::icon::{Icon, IconStroke};
use crate::icon_path::IconPath;

const WIDTH: f32 = 10.0;

fn pt(x: f32, y: f32) -> Point {
    Point::new(x, y)
}

/// A point `length` from the origin at `degrees` from the x axis.
fn polar(length: f32, degrees: f32) -> Point {
    let radians = degrees.to_radians();
    pt(length * radians.cos(), length * radians.sin())
}

/// The path in from (100, 0) to the origin and out at `degrees` to the x axis.
fn corner(degrees: f32) -> Vec<Command> {
    vec![
        Command::MoveTo(pt(100.0, 0.0)),
        Command::LineTo(pt(0.0, 0.0)),
        Command::LineTo(polar(100.0, degrees)),
    ]
}

fn wedge_pivots(wedges: &[Command]) -> Vec<Point> {
    wedges
        .iter()
        .filter_map(|c| match c {
            Command::MoveTo(p) => Some(*p),
            _ => None,
        })
        .collect()
}

fn all_finite(commands: &[Command]) -> bool {
    commands.iter().all(|c| match *c {
        Command::MoveTo(p) | Command::LineTo(p) => p.x.is_finite() && p.y.is_finite(),
        _ => true,
    })
}

#[test]
fn a_join_at_exactly_90_degrees_adds_no_wedge() {
    // Given: in along -x, out along +y
    let commands = [
        Command::MoveTo(pt(100.0, 0.0)),
        Command::LineTo(pt(0.0, 0.0)),
        Command::LineTo(pt(0.0, 100.0)),
    ];

    // When
    let wedges = missing_wedges(&commands, WIDTH);

    // Then
    assert!(wedges.is_empty());
}

#[test]
fn a_join_of_29_degrees_adds_a_wedge_since_its_miter_ratio_is_3_99() {
    // Given / When
    let wedges = missing_wedges(&corner(29.0), WIDTH);

    // Then
    assert_eq!(wedge_pivots(&wedges), [pt(0.0, 0.0)]);
}

#[test]
fn a_join_of_28_degrees_adds_no_wedge_since_its_miter_ratio_is_4_13() {
    // Given / When
    let wedges = missing_wedges(&corner(28.0), WIDTH);

    // Then
    assert!(wedges.is_empty());
}

#[test]
fn a_closed_paths_join_at_its_start_gets_a_wedge() {
    // Given: a 40-degree apex at the start, closed
    let mut commands = vec![
        Command::MoveTo(pt(0.0, 0.0)),
        Command::LineTo(pt(100.0, 0.0)),
        Command::LineTo(polar(100.0, 40.0)),
    ];
    let open = missing_wedges(&commands, WIDTH);
    commands.push(Command::Close);

    // When
    let closed = missing_wedges(&commands, WIDTH);

    // Then
    assert!(!wedge_pivots(&open).contains(&pt(0.0, 0.0)));
    assert!(wedge_pivots(&closed).contains(&pt(0.0, 0.0)));
}

#[test]
fn a_curves_end_tangent_decides_the_join_after_it() {
    // Given: the curve arrives along -x though its chord points down-left
    let commands = [
        Command::MoveTo(pt(100.0, 60.0)),
        Command::CurveTo(pt(100.0, 30.0), pt(100.0, 0.0), pt(0.0, 0.0)),
        Command::LineTo(polar(100.0, 40.0)),
    ];

    // When
    let wedges = missing_wedges(&commands, WIDTH);

    // Then
    assert_eq!(wedge_pivots(&wedges), [pt(0.0, 0.0)]);
}

#[test]
fn a_degenerate_curve_skips_its_join_without_a_non_finite_point() {
    // Given: a curve whose control points sit on its end point
    let at = pt(100.0, 0.0);
    let with_curve = [
        Command::MoveTo(pt(0.0, 0.0)),
        Command::LineTo(at),
        Command::CurveTo(at, at, at),
        Command::LineTo(polar(100.0, 140.0) + pt(100.0, 0.0)),
    ];
    let without_curve = [with_curve[0], with_curve[1], with_curve[3]];

    // When
    let wedges = missing_wedges(&with_curve, WIDTH);

    // Then
    assert!(!wedges.is_empty() && all_finite(&wedges));
    assert_eq!(wedges, missing_wedges(&without_curve, WIDTH));
}

#[test]
fn a_curve_whose_control_points_sit_on_its_end_joins_as_a_line() {
    // Given: the same path with a line in place of the curve
    let end = pt(100.0, 0.0);
    let corner_back = polar(100.0, 140.0) + end;
    let curved = [
        Command::MoveTo(pt(0.0, 0.0)),
        Command::CurveTo(end, end, end),
        Command::LineTo(corner_back),
    ];
    let straight = [curved[0], Command::LineTo(end), curved[2]];

    // When
    let wedges = missing_wedges(&curved, WIDTH);

    // Then
    assert_eq!(wedge_pivots(&wedges), [end]);
    assert_eq!(wedges, missing_wedges(&straight, WIDTH));
}

#[test]
fn a_reversal_on_a_diagonal_adds_no_wedge_and_rasterizes() {
    // Given: dot is -1.0000001 here, whose square root of the midpoint is NaN
    let path = IconPath::parse("M10 10 L100 100 L10 10").expect("parses");
    let commands = path.commands();

    // When
    let wedges = missing_wedges(commands, 20.0);
    let stroke = IconStroke { path, width: 20.0 };
    let icon = Icon::new([0.0, 0.0, 200.0, 200.0], vec![stroke]).expect("a valid icon");
    let coverage = icon.rasterize([200, 200]);

    // Then
    assert!(wedges.is_empty());
    assert!(coverage.data.iter().any(|&a| a > 0));
}
