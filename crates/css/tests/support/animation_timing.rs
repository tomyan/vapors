//! Helpers that were in `tests/animation_timing.rs`.
#![allow(dead_code)]

pub fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 0.001
}

/// Reference solve of x(u) = t by bisection to 1e-9.
pub fn bisect_bezier(x1: f64, y1: f64, x2: f64, y2: f64, t: f64) -> f64 {
    let axis = |p1: f64, p2: f64, u: f64| {
        let inv = 1.0 - u;
        3.0 * inv * inv * u * p1 + 3.0 * inv * u * u * p2 + u * u * u
    };
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..60 {
        let mid = f64::midpoint(lo, hi);
        if axis(x1, x2, mid) < t {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    axis(y1, y2, f64::midpoint(lo, hi))
}
