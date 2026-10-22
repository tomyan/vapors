//! Timing function parsing (`src/animation/timing.rs`). Translated from sumi `easing_test.go` and `steps_test.go`.

mod support;

use vapors_css::animation::{StepPosition, TimingFunction};

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
