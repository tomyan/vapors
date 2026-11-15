//! A stroked round rect covers what its path stroked covers (#387
//! review, L3). A stroke covers every point within half its width of
//! the outline, so the two agree, at any width, when the distance to
//! the closed form's outline is the distance to the path's. That
//! includes a stroke wider than the corners, whose inner edge has sharp
//! corners. The closed form's distance is `round_corners`' formula in
//! `closed_forms.wgsl`. The path's is taken to its outline as zeno
//! draws it: lines, and arcs as cubics within 2.7e-4 of the radius,
//! each cubic cut into 250 lines. Elliptical corners are held to the
//! exact distance by the renderer's sweep (`gpu_closed_forms_exact.rs`).

use zeno::{Command, Point};

use crate::canonical::Paint;
use crate::classify::{Classified, classify};
use crate::path::IconPath;

/// The outline as points along it, closed.
fn outline(path: &IconPath) -> Vec<Point> {
    let mut points: Vec<Point> = Vec::new();
    for command in path.commands() {
        match *command {
            Command::MoveTo(to) | Command::LineTo(to) => points.push(to),
            Command::CurveTo(a, b, to) => points.extend(cut(points[points.len() - 1], a, b, to)),
            Command::QuadTo(..) => panic!("an arc is drawn as cubics"),
            Command::Close => points.push(points[0]),
        }
    }
    points
}

/// The cubic from `from` through `a` and `b` to `to`, as 250 points.
fn cut(from: Point, a: Point, b: Point, to: Point) -> Vec<Point> {
    (1..=250u16)
        .map(|i| {
            let t = f32::from(i) / 250.0;
            let u = 1.0 - t;
            let weights = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
            let along = |pick: fn(Point) -> f32| {
                weights[0] * pick(from)
                    + weights[1] * pick(a)
                    + weights[2] * pick(b)
                    + weights[3] * pick(to)
            };
            Point::new(along(|p| p.x), along(|p| p.y))
        })
        .collect()
}

/// The distance from `p` to the polyline through `points`.
fn to_polyline(points: &[Point], p: Point) -> f32 {
    points
        .windows(2)
        .map(|pair| {
            let (a, b) = (pair[0], pair[1]);
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            let length = dx * dx + dy * dy;
            let t = if length > 0.0 {
                (((p.x - a.x) * dx + (p.y - a.y) * dy) / length).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (p.x - a.x - t * dx).hypot(p.y - a.y - t * dy)
        })
        .fold(f32::INFINITY, f32::min)
}

/// The distance from `p` to the outline of the rect at `origin` of
/// `size` with circular corners of `radius`, unsigned.
fn to_round_rect(origin: [f32; 2], size: [f32; 2], radius: f32, p: Point) -> f32 {
    let q = [(p.x, 0), (p.y, 1)]
        .map(|(v, i)| (v - origin[i] - size[i] / 2.0).abs() - size[i] / 2.0 + radius);
    let outer = q[0].max(0.0).hypot(q[1].max(0.0));
    (outer + q[0].max(q[1]).min(0.0) - radius).abs()
}

#[test]
fn a_stroked_round_rect_is_as_far_from_every_point_as_its_path() {
    // Given: corners of radius 2, classified as a stroke 5 wide, wider
    // than both corners
    let data = "M4 3H10A2 2 0 0 1 12 5V9A2 2 0 0 1 10 11H4A2 2 0 0 1 2 9V5A2 2 0 0 1 4 3Z";
    let path = IconPath::parse(data).expect("parses");
    let Classified::Rect {
        origin,
        size,
        radius,
    } = classify(&path, Paint::Stroke { width: 5.0 })
    else {
        panic!("{data} is not a rect");
    };
    let points = outline(&path);

    // When: every 0.1 units over the box and 3 units round it
    let mut worst = (0.0, Point::new(0.0, 0.0));
    for (i, j) in (0..=160u16).flat_map(|i| (0..=140u16).map(move |j| (i, j))) {
        let p = Point::new(-1.0 + f32::from(i) * 0.1, f32::from(j) * 0.1);
        let off = (to_polyline(&points, p) - to_round_rect(origin, size, radius[0], p)).abs();
        if off > worst.0 {
            worst = (off, p);
        }
    }

    // Then: within zeno's cubic arcs (5.4e-4 at radius 2) and the cuts
    assert!(worst.0 <= 1e-3, "{} off at {:?}", worst.0, worst.1);
}
