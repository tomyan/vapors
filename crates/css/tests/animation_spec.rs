//! Specs from a computed style (`src/animation/spec.rs`). Translated from sumi `transition_parse_test.go` and `animation_parse_test.go`.

mod support;

use support::animation_specs::{animations_of, property, transitions_of};
use support::chain;
use vapors_css::animation::{
    Direction, FillMode, IterationCount, PlayState, TimingFunction, TransitionProperty, animations,
};

#[test]
fn no_animation_leaves_the_list_empty() {
    let (tree, leaf) = chain(&["div"]);
    let style = support::computed("div { color: red; }", &tree, leaf);
    assert_eq!(animations(&style), []);
}

#[test]
fn parse_transition_shorthand() {
    // When
    let specs = transitions_of("transition: color 200ms ease-out;");

    // Then
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0].property, property("color"));
    assert_eq!(specs[0].duration_ms, 200.0);
    assert_eq!(specs[0].timing, TimingFunction::EASE_OUT);
    assert_eq!(specs[0].delay_ms, 0.0);
}

#[test]
fn parse_transition_with_delay() {
    assert_eq!(
        transitions_of("transition: color 200ms ease-out 50ms;")[0].delay_ms,
        50.0
    );
}

#[test]
fn parse_transition_multiple() {
    let specs = transitions_of("transition: color 200ms ease-out, background-color 500ms ease-in;");
    assert_eq!(specs.len(), 2);
    assert_eq!(
        (specs[0].property.clone(), specs[0].duration_ms),
        (property("color"), 200.0)
    );
    assert_eq!(
        (specs[1].property.clone(), specs[1].duration_ms),
        (property("background-color"), 500.0)
    );
}

#[test]
fn parse_transition_all() {
    let specs = transitions_of("transition: all 300ms linear;");
    assert_eq!(specs[0].property, TransitionProperty::All);
    assert_eq!(specs[0].timing, TimingFunction::LINEAR);
}

#[test]
fn parse_transition_duration_seconds() {
    assert_eq!(
        transitions_of("transition: color 0.5s ease;")[0].duration_ms,
        500.0
    );
    assert_eq!(
        transitions_of("transition: color 2s ease;")[0].duration_ms,
        2000.0
    );
}

#[test]
fn parse_transition_default_easing() {
    assert_eq!(
        transitions_of("transition: color 200ms;")[0].timing,
        TimingFunction::EASE
    );
}

#[test]
fn no_transition_declared_means_none() {
    // Given: the initial transition-property is `all` with a 0s duration,
    // which is no transition at all.
    assert_eq!(transitions_of("color: red;"), []);
    assert_eq!(transitions_of("transition: none;"), []);
}

#[test]
fn parse_transition_longhand() {
    let specs = transitions_of(
        "transition-property: color; transition-duration: 200ms; transition-timing-function: ease-in; transition-delay: 100ms;",
    );
    assert_eq!(specs.len(), 1);
    let s = &specs[0];
    assert_eq!(
        (s.property.clone(), s.duration_ms, s.timing, s.delay_ms),
        (property("color"), 200.0, TimingFunction::EASE_IN, 100.0)
    );
}

#[test]
fn shorter_longhand_lists_repeat_as_the_specification_says() {
    let specs =
        transitions_of("transition-property: color, opacity, width; transition-duration: 1s, 2s;");
    let durations: Vec<f64> = specs.iter().map(|s| s.duration_ms).collect();
    assert_eq!(durations, [1000.0, 2000.0, 1000.0]);
}

#[test]
fn unknown_transition_property_is_refused_at_parse() {
    let err = support::refusal(".x { transition: colour 1s; }");
    assert!(err.message.contains("colour"), "{err}");
}

#[test]
fn parse_animation_shorthand() {
    // When
    let specs = animations_of("animation: pulse 2s infinite ease-in-out;");

    // Then
    assert_eq!(specs.len(), 1);
    let s = &specs[0];
    assert_eq!(s.name, "pulse");
    assert_eq!(s.duration_ms, 2000.0);
    assert_eq!(s.iteration_count, IterationCount::Infinite);
    assert_eq!(s.timing, TimingFunction::EASE_IN_OUT);
}

#[test]
fn parse_animation_minimal() {
    let s = &animations_of("animation: fade 500ms;")[0];
    assert_eq!(
        (s.name.as_str(), s.duration_ms, s.iteration_count),
        ("fade", 500.0, IterationCount::Finite(1.0))
    );
    assert_eq!(
        (s.direction, s.fill_mode, s.play_state),
        (Direction::Normal, FillMode::None, PlayState::Running)
    );
}

#[test]
fn parse_animation_with_direction() {
    let s = &animations_of("animation: bounce 1s alternate 3;")[0];
    assert_eq!(s.direction, Direction::Alternate);
    assert_eq!(s.iteration_count, IterationCount::Finite(3.0));
}

#[test]
fn parse_animation_empty() {
    assert_eq!(animations_of("color: red;"), []);
    assert_eq!(animations_of("animation: none;"), []);
}

#[test]
fn parse_animation_longhand() {
    let s = &animations_of(
        "animation-name: pulse; animation-duration: 2s; animation-timing-function: ease-in; animation-iteration-count: infinite; animation-direction: alternate; animation-fill-mode: forwards;",
    )[0];
    assert_eq!(
        (s.name.as_str(), s.duration_ms, s.iteration_count),
        ("pulse", 2000.0, IterationCount::Infinite)
    );
    assert_eq!(
        (s.direction, s.fill_mode),
        (Direction::Alternate, FillMode::Forwards)
    );
}
