//! Transition sampling (`src/animation/transition.rs`).

mod support;

use support::animation_sample::px;
use vapors_css::animation::{TimingFunction, Transition, TransitionProperty, TransitionSpec};
use vapors_css::property::PropertyId;

#[test]
fn transition_sample_follows_delay_duration_and_easing() {
    // Given
    let spec = TransitionSpec {
        property: TransitionProperty::Property(PropertyId::from_name("width").expect("width")),
        duration_ms: 100.0,
        timing: TimingFunction::LINEAR,
        delay_ms: 50.0,
    };
    let t = Transition::new(spec, px(0.0), px(100.0));

    // Then
    assert_eq!(t.sample(0.0), (px(0.0), false));
    assert_eq!(t.sample(50.0), (px(0.0), false));
    assert_eq!(t.sample(100.0), (px(50.0), false));
    assert_eq!(t.sample(150.0), (px(100.0), true));
    assert_eq!(t.sample(999.0), (px(100.0), true));
}

#[test]
fn transition_retargets_from_the_current_value() {
    let spec = TransitionSpec {
        property: TransitionProperty::All,
        duration_ms: 100.0,
        timing: TimingFunction::LINEAR,
        delay_ms: 0.0,
    };
    let t = Transition::new(spec.clone(), px(0.0), px(100.0));
    let (current, _) = t.sample(40.0);
    let retargeted = Transition::new(spec, current, px(0.0));
    assert_eq!(retargeted.sample(20.0), (px(32.0), false));
}
