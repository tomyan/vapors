//! Timing functions. Translated from sumi's `runtime/anim/easing_test.go`
//! and `steps_test.go`.

use vapors_css::animation::{StepPosition, TimingFunction};

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 0.001
}

#[test]
fn linear_easing() {
    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        assert!(near(TimingFunction::LINEAR.evaluate(t), t), "linear({t})");
    }
}

#[test]
fn easing_boundary_values() {
    for (name, f) in [
        ("linear", TimingFunction::LINEAR),
        ("ease", TimingFunction::EASE),
        ("ease-in", TimingFunction::EASE_IN),
        ("ease-out", TimingFunction::EASE_OUT),
        ("ease-in-out", TimingFunction::EASE_IN_OUT),
    ] {
        assert!(near(f.evaluate(0.0), 0.0), "{name}(0)");
        assert!(near(f.evaluate(1.0), 1.0), "{name}(1)");
    }
}

#[test]
fn easing_monotonic() {
    for (name, f) in [
        ("ease", TimingFunction::EASE),
        ("ease-in", TimingFunction::EASE_IN),
        ("ease-out", TimingFunction::EASE_OUT),
        ("ease-in-out", TimingFunction::EASE_IN_OUT),
    ] {
        let mut prev = 0.0;
        for i in 1..=100 {
            let t = f64::from(i) / 100.0;
            let got = f.evaluate(t);
            assert!(
                got >= prev - 0.001,
                "{name} not monotonic at {t}: {got} < {prev}"
            );
            prev = got;
        }
    }
}

#[test]
fn ease_in_slower_at_start_and_ease_out_faster() {
    assert!(TimingFunction::EASE_IN.evaluate(0.25) < 0.25);
    assert!(TimingFunction::EASE_OUT.evaluate(0.25) > 0.25);
}

#[test]
fn parse_timing_function_names() {
    for (text, want) in [
        ("linear", TimingFunction::LINEAR),
        ("ease", TimingFunction::EASE),
        ("ease-in", TimingFunction::EASE_IN),
        ("ease-out", TimingFunction::EASE_OUT),
        ("ease-in-out", TimingFunction::EASE_IN_OUT),
        (
            "step-start",
            TimingFunction::Steps {
                count: 1,
                position: StepPosition::Start,
            },
        ),
        (
            "step-end",
            TimingFunction::Steps {
                count: 1,
                position: StepPosition::End,
            },
        ),
    ] {
        assert_eq!(
            TimingFunction::parse(text).unwrap_or_else(|e| panic!("{text}: {e}")),
            want,
            "{text}"
        );
    }
}

#[test]
fn parse_cubic_bezier_linear_control_points() {
    let f = TimingFunction::parse("cubic-bezier(0, 0, 1, 1)").expect("parses");
    for t in [0.25, 0.5, 0.75] {
        assert!(
            (f.evaluate(t) - t).abs() < 0.01,
            "cubic-bezier(0,0,1,1)({t})"
        );
    }
}

#[test]
fn cubic_bezier_solves_x_for_t_not_the_parameter() {
    // Given: ease at x = 0.5 is about 0.8 (the Newton solve matters).
    let got = TimingFunction::EASE.evaluate(0.5);
    assert!((got - 0.802).abs() < 0.01, "ease(0.5) = {got}");
}

#[test]
fn parse_timing_function_errors() {
    for text in [
        "bogus",
        "cubic-bezier(1, 2, 3)",
        "cubic-bezier(2, 0, 1, 1)",
        "steps(0)",
        "steps(2, sideways)",
    ] {
        assert!(TimingFunction::parse(text).is_err(), "{text}");
    }
}

#[test]
fn parse_steps() {
    assert_eq!(
        TimingFunction::parse("steps(4)").expect("parses"),
        TimingFunction::Steps {
            count: 4,
            position: StepPosition::End
        }
    );
    assert_eq!(
        TimingFunction::parse("steps(3, start)").expect("parses"),
        TimingFunction::Steps {
            count: 3,
            position: StepPosition::Start
        }
    );
    assert_eq!(
        TimingFunction::parse("steps(3, jump-none)").expect("parses"),
        TimingFunction::Steps {
            count: 3,
            position: StepPosition::JumpNone
        }
    );
}

#[test]
fn step_end_evaluation() {
    let f = TimingFunction::parse("steps(4)").expect("parses");
    for (t, want) in [
        (0.0, 0.0),
        (0.1, 0.0),
        (0.26, 0.25),
        (0.6, 0.5),
        (0.99, 0.75),
        (1.0, 1.0),
    ] {
        assert_eq!(f.evaluate(t), want, "steps(4)({t})");
    }
}

#[test]
fn step_start_evaluation() {
    assert_eq!(
        TimingFunction::parse("step-start")
            .expect("parses")
            .evaluate(0.01),
        1.0
    );
    assert_eq!(
        TimingFunction::parse("step-end")
            .expect("parses")
            .evaluate(0.99),
        0.0
    );
}

#[test]
fn jump_none_and_jump_both_evaluation() {
    // jump-none: 3 steps hold 0, 0.5, 1 over the interval.
    let f = TimingFunction::parse("steps(3, jump-none)").expect("parses");
    assert_eq!(
        (f.evaluate(0.1), f.evaluate(0.5), f.evaluate(0.9)),
        (0.0, 0.5, 1.0)
    );
    // jump-both: 2 steps give 1/3 and 2/3 with jumps at both ends.
    let f = TimingFunction::parse("steps(2, jump-both)").expect("parses");
    assert!(
        (f.evaluate(0.1) - 1.0 / 3.0).abs() < 1e-9 && (f.evaluate(0.6) - 2.0 / 3.0).abs() < 1e-9
    );
}

/// Reference solve of x(u) = t by bisection to 1e-9.
fn bisect_bezier(x1: f64, y1: f64, x2: f64, y2: f64, t: f64) -> f64 {
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

#[test]
fn cubic_bezier_with_a_flat_middle_matches_a_bisection_reference() {
    // Given: review item 5; Newton alone stalls where the derivative is small.
    let f = TimingFunction::parse("cubic-bezier(1, 0, 0, 1)").expect("parses");
    for t in [0.1, 0.3, 0.5, 0.7, 0.9] {
        let want = bisect_bezier(1.0, 0.0, 0.0, 1.0, t);
        assert!(
            (f.evaluate(t) - want).abs() < 1e-4,
            "t={t}: {} vs {want}",
            f.evaluate(t)
        );
    }
}
