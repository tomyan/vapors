//! The curve maths behind timing functions: the cubic Bézier solve and
//! the step function.

use super::timing::StepPosition;

/// Solve `B_x(u) = t` by Newton iteration, falling back to bisection
/// when the derivative is small or a step leaves 0..=1, then return
/// `B_y(u)`.
pub(super) fn bezier_y(x1: f64, y1: f64, x2: f64, y2: f64, t: f64) -> f64 {
    let u = newton(x1, x2, t).unwrap_or_else(|| bisect(x1, x2, t));
    bezier(y1, y2, u)
}

/// Eight Newton steps; `None` when they do not converge.
fn newton(x1: f64, x2: f64, t: f64) -> Option<f64> {
    let mut u = t;
    for _ in 0..8 {
        let error = bezier(x1, x2, u) - t;
        if error.abs() < 1e-7 {
            return Some(u);
        }
        let slope = bezier_derivative(x1, x2, u);
        if slope.abs() < 1e-6 {
            return None;
        }
        u -= error / slope;
        if !(0.0..=1.0).contains(&u) {
            return None;
        }
    }
    ((bezier(x1, x2, u) - t).abs() < 1e-6).then_some(u)
}

/// Bisection on 0..=1: `B_x` is monotonic there since x1 and x2 are.
fn bisect(x1: f64, x2: f64, t: f64) -> f64 {
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..50 {
        let mid = f64::midpoint(lo, hi);
        if bezier(x1, x2, mid) < t {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    f64::midpoint(lo, hi)
}

/// One axis of the curve: `3(1-u)²u·p1 + 3(1-u)u²·p2 + u³`.
fn bezier(p1: f64, p2: f64, u: f64) -> f64 {
    let inv = 1.0 - u;
    3.0 * inv * inv * u * p1 + 3.0 * inv * u * u * p2 + u * u * u
}

fn bezier_derivative(p1: f64, p2: f64, u: f64) -> f64 {
    let inv = 1.0 - u;
    3.0 * inv * inv * p1 + 6.0 * inv * u * (p2 - p1) + 3.0 * u * u * (1.0 - p2)
}

/// Easing Level 1 section 3.2.3.
pub(super) fn steps(t: f64, count: u32, position: StepPosition) -> f64 {
    let n = f64::from(count);
    let mut step = (t * n).floor();
    if matches!(position, StepPosition::Start | StepPosition::JumpBoth) {
        step += 1.0;
    }
    let jumps = match position {
        StepPosition::JumpNone => n - 1.0,
        StepPosition::JumpBoth => n + 1.0,
        StepPosition::Start | StepPosition::End => n,
    };
    (step / jumps).clamp(0.0, 1.0)
}
