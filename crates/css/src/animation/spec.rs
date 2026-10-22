//! Transition and animation specs read from a computed style's
//! `transition-*` and `animation-*` longhands. Shorter lists repeat
//! (Transitions Level 1 section 2.1).

use super::timing::TimingFunction;
use crate::cascade::ComputedStyle;
use crate::property::PropertyId;
use crate::value::Value;

/// What a transition applies to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransitionProperty {
    /// `all`
    All,
    /// One longhand (shorthands are expanded to their longhands).
    Property(PropertyId),
}

/// One `transition-*` entry.
#[derive(Clone, Debug, PartialEq)]
pub struct TransitionSpec {
    /// What it applies to.
    pub property: TransitionProperty,
    /// Duration in milliseconds.
    pub duration_ms: f64,
    /// Easing.
    pub timing: TimingFunction,
    /// Delay in milliseconds.
    pub delay_ms: f64,
}

/// `animation-iteration-count`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IterationCount {
    /// A number of iterations, fractional allowed.
    Finite(f64),
    /// `infinite`
    Infinite,
}

/// `animation-direction`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// Forwards every iteration.
    Normal,
    /// Backwards every iteration.
    Reverse,
    /// Forwards, then backwards, alternating.
    Alternate,
    /// Backwards, then forwards, alternating.
    AlternateReverse,
}

/// `animation-fill-mode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FillMode {
    /// No values outside the active interval.
    None,
    /// Hold the last values after the end.
    Forwards,
    /// Hold the first values during the delay.
    Backwards,
    /// Both.
    Both,
}

/// `animation-play-state`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayState {
    /// Advancing.
    Running,
    /// Frozen; the scene layer stops advancing elapsed time.
    Paused,
}

/// One `animation-*` entry.
#[derive(Clone, Debug, PartialEq)]
pub struct AnimationSpec {
    /// The `@keyframes` name.
    pub name: String,
    /// Duration of one iteration in milliseconds.
    pub duration_ms: f64,
    /// Easing per keyframe segment.
    pub timing: TimingFunction,
    /// Delay in milliseconds.
    pub delay_ms: f64,
    /// How many iterations.
    pub iteration_count: IterationCount,
    /// Direction.
    pub direction: Direction,
    /// Fill mode.
    pub fill_mode: FillMode,
    /// Play state.
    pub play_state: PlayState,
}

fn items<'a>(style: &'a ComputedStyle, name: &str) -> &'a [Value] {
    let id = PropertyId::from_name(name).expect("table lists the animation longhands");
    match style.get(id) {
        Value::CommaList(items) => items,
        single => std::slice::from_ref(single),
    }
}

/// The `i`th item of a repeating list.
fn cycled(list: &[Value], i: usize) -> &Value {
    &list[i % list.len()]
}

fn millis(v: &Value) -> f64 {
    match v {
        Value::Time(t) => t.millis(),
        _ => 0.0,
    }
}

fn timing(v: &Value) -> TimingFunction {
    TimingFunction::from_value(v).unwrap_or(TimingFunction::EASE)
}

/// The transitions a style declares, with 0ms ones (the default `all 0s`)
/// left out since they never run.
#[must_use]
pub fn transitions(style: &ComputedStyle) -> Vec<TransitionSpec> {
    let properties = items(style, "transition-property");
    let durations = items(style, "transition-duration");
    let timings = items(style, "transition-timing-function");
    let delays = items(style, "transition-delay");
    let mut out = Vec::new();
    for (i, property) in properties.iter().enumerate() {
        let duration_ms = millis(cycled(durations, i));
        if duration_ms <= 0.0 {
            continue;
        }
        for property in transition_properties(property) {
            out.push(TransitionSpec {
                property,
                duration_ms,
                timing: timing(cycled(timings, i)),
                delay_ms: millis(cycled(delays, i)),
            });
        }
    }
    out
}

/// `all`, a longhand, or every longhand of a shorthand.
fn transition_properties(value: &Value) -> Vec<TransitionProperty> {
    let Some(name) = value.as_keyword() else {
        return Vec::new();
    };
    match name {
        "all" => vec![TransitionProperty::All],
        "none" => Vec::new(),
        name => match PropertyId::from_name(name) {
            Some(id) => vec![TransitionProperty::Property(id)],
            None => crate::property::SHORTHANDS
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, longhands)| {
                    longhands
                        .iter()
                        .filter_map(|l| PropertyId::from_name(l))
                        .map(TransitionProperty::Property)
                        .collect()
                })
                .unwrap_or_default(),
        },
    }
}

/// The animations a style declares, `none` entries left out.
#[must_use]
pub fn animations(style: &ComputedStyle) -> Vec<AnimationSpec> {
    let names = items(style, "animation-name");
    let mut out = Vec::new();
    for (i, name) in names.iter().enumerate() {
        let Some(name) = name.as_keyword().filter(|n| *n != "none") else {
            continue;
        };
        out.push(AnimationSpec {
            name: name.to_owned(),
            duration_ms: millis(cycled(items(style, "animation-duration"), i)),
            timing: timing(cycled(items(style, "animation-timing-function"), i)),
            delay_ms: millis(cycled(items(style, "animation-delay"), i)),
            iteration_count: match cycled(items(style, "animation-iteration-count"), i) {
                Value::Number(n) => IterationCount::Finite(*n),
                _ => IterationCount::Infinite,
            },
            direction: match cycled(items(style, "animation-direction"), i).as_keyword() {
                Some("reverse") => Direction::Reverse,
                Some("alternate") => Direction::Alternate,
                Some("alternate-reverse") => Direction::AlternateReverse,
                _ => Direction::Normal,
            },
            fill_mode: match cycled(items(style, "animation-fill-mode"), i).as_keyword() {
                Some("forwards") => FillMode::Forwards,
                Some("backwards") => FillMode::Backwards,
                Some("both") => FillMode::Both,
                _ => FillMode::None,
            },
            play_state: match cycled(items(style, "animation-play-state"), i).as_keyword() {
                Some("paused") => PlayState::Paused,
                _ => PlayState::Running,
            },
        });
    }
    out
}
