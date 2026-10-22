//! Animation sampling (`src/animation/sampler.rs`).

mod support;

use support::animation_sample::{pulse_animation, px};
use vapors_css::property::PropertyId;
use vapors_css::value::ColorScheme;

#[test]
fn animation_sample_iterations_direction_and_fill() {
    // Given: 0 to 100px over 100ms, 2 iterations, alternate, delay 50, fill both.
    let src = ".pulse { width: 7px; animation: grow 100ms linear 50ms 2 alternate both; } @keyframes grow { from { width: 0px; } to { width: 100px; } }";
    let anim = pulse_animation(src, ColorScheme::Light);
    let width = PropertyId::from_name("width").expect("width");
    let at = |ms: f64| {
        let s = anim.sample(ms);
        (
            s.values
                .into_iter()
                .find(|(id, _)| *id == width)
                .map(|(_, v)| v),
            s.finished,
        )
    };

    // Then
    assert_eq!(
        at(0.0),
        (Some(px(0.0)), false),
        "backwards fill during the delay"
    );
    assert_eq!(at(75.0), (Some(px(25.0)), false));
    assert_eq!(at(150.0), (Some(px(100.0)), false), "end of iteration 1");
    assert_eq!(
        at(200.0),
        (Some(px(50.0)), false),
        "iteration 2 runs backwards"
    );
    assert_eq!(
        at(250.0),
        (Some(px(0.0)), true),
        "forwards fill holds the last value"
    );
    assert_eq!(at(9999.0), (Some(px(0.0)), true));
}

#[test]
fn animation_without_fill_holds_nothing_outside_its_interval() {
    let src = ".pulse { animation: grow 100ms linear; } @keyframes grow { from { width: 0px; } to { width: 100px; } }";
    let anim = pulse_animation(src, ColorScheme::Light);
    assert_eq!(anim.sample(150.0).values, []);
    assert!(anim.sample(150.0).finished);
    assert_ne!(anim.sample(50.0).values, []);
}

#[test]
fn keyframe_timing_applies_per_segment() {
    let src = ".pulse { animation: s 100ms steps(1, end); } @keyframes s { from { width: 0px; } 50% { width: 50px; } to { width: 100px; } }";
    let anim = pulse_animation(src, ColorScheme::Light);
    let width = PropertyId::from_name("width").expect("width");
    let at = |ms: f64| {
        anim.sample(ms)
            .values
            .into_iter()
            .find(|(id, _)| *id == width)
            .map(|(_, v)| v)
    };
    assert_eq!(at(25.0), Some(px(0.0)));
    assert_eq!(at(75.0), Some(px(50.0)));
}
