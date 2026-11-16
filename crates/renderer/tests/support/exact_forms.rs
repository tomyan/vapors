//! The exact signed distance to a closed form and its gradient, in f64,
//! by search rather than by the shader's refinement: the reference the
//! sweep holds the GPU to. The distance is to the nearest piece of the
//! outline: an edge segment, or a quarter ellipse found by a dense
//! search narrowed by ternary search.

// each test crate including this uses a part of it
#![allow(dead_code)]

use vapors_shapes::Geometry;

/// The distance from `p` to `geometry`'s outline, negative inside, and
/// the outward normal at the nearest point (a unit vector), in its own
/// units.
pub fn exact(geometry: &Geometry, p: [f64; 2]) -> (f64, [f64; 2]) {
    match *geometry {
        Geometry::Rect {
            origin,
            size,
            radius,
        } => rect(
            origin.map(f64::from),
            size.map(f64::from),
            radius.map(f64::from),
            p,
        ),
        Geometry::Ellipse { centre, radii } => {
            ellipse(centre.map(f64::from), radii.map(f64::from), p)
        }
        Geometry::Mesh { .. } => panic!("a mesh is not a closed form"),
    }
}

fn rect(origin: [f64; 2], size: [f64; 2], radii: [f64; 2], p: [f64; 2]) -> (f64, [f64; 2]) {
    let half = size.map(|v| v / 2.0);
    let from_centre = [0, 1].map(|i| p[i] - origin[i] - half[i]);
    let folded = from_centre.map(f64::abs);
    let corner = [0, 1].map(|i| half[i] - radii[i]);
    let q = [0, 1].map(|i| folded[i] - corner[i]);
    let top = (q[0].max(0.0)).hypot(folded[1] - half[1]);
    let to_side = (folded[0] - half[0]).hypot(q[1].max(0.0));
    let (t, arc) = nearest_on_quarter(q, radii);
    let (d, normal) = [
        (top, [0.0, 1.0]),
        (to_side, [1.0, 0.0]),
        (arc, normal_at(t, radii)),
    ]
    .into_iter()
    .fold((f64::INFINITY, [0.0; 2]), |best, piece| {
        if piece.0 < best.0 { piece } else { best }
    });
    let past_arc = q[0] > 0.0 && q[1] > 0.0 && (q[0] / radii[0]).hypot(q[1] / radii[1]) > 1.0;
    let inside = folded[0] < half[0] && folded[1] < half[1] && !past_arc;
    (if inside { -d } else { d }, unfold(normal, from_centre))
}

fn ellipse(centre: [f64; 2], radii: [f64; 2], p: [f64; 2]) -> (f64, [f64; 2]) {
    let q = [p[0] - centre[0], p[1] - centre[1]];
    let folded = q.map(f64::abs);
    let (t, d) = nearest_on_quarter(folded, radii);
    let inside = (folded[0] / radii[0]).hypot(folded[1] / radii[1]) < 1.0;
    (if inside { -d } else { d }, unfold(normal_at(t, radii), q))
}

/// The angle of the point of the quarter ellipse (x, y >= 0) nearest
/// `q`, and its distance.
fn nearest_on_quarter(q: [f64; 2], radii: [f64; 2]) -> (f64, f64) {
    let distance = |t: f64| (q[0] - radii[0] * t.cos()).hypot(q[1] - radii[1] * t.sin());
    let step = std::f64::consts::FRAC_PI_2 / 2000.0;
    let start = (0..=2000)
        .map(|i| f64::from(i) * step)
        .fold(0.0, |best, t| {
            if distance(t) < distance(best) {
                t
            } else {
                best
            }
        });
    let (mut lo, mut hi) = (
        (start - step).max(0.0),
        (start + step).min(2.0 * step * 1000.0),
    );
    for _ in 0..60 {
        let (a, b) = (lo + (hi - lo) / 3.0, hi - (hi - lo) / 3.0);
        if distance(a) < distance(b) {
            hi = b;
        } else {
            lo = a;
        }
    }
    let t = f64::midpoint(lo, hi);
    (t, distance(t))
}

fn normal_at(t: f64, radii: [f64; 2]) -> [f64; 2] {
    let normal = [radii[1] * t.cos(), radii[0] * t.sin()];
    let length = normal[0].hypot(normal[1]);
    normal.map(|v| v / length)
}

/// `normal`, found in the folded quadrant, back in `towards`'s.
fn unfold(normal: [f64; 2], towards: [f64; 2]) -> [f64; 2] {
    [0, 1].map(|i| {
        if towards[i] < 0.0 {
            -normal[i]
        } else {
            normal[i]
        }
    })
}
