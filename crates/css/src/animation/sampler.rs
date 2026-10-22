//! Sampling an animation by elapsed time (Animations Level 1 section 3).

use super::interpolate::interpolate;
use super::keyframes::{ResolvedKeyframes, Stop};
use super::spec::{AnimationSpec, Direction, FillMode, IterationCount};
use crate::property::PropertyId;
use crate::value::Value;

/// A keyframe animation on one element.
#[derive(Clone, Debug, PartialEq)]
pub struct Animation {
    spec: AnimationSpec,
    keyframes: ResolvedKeyframes,
}

/// The values an animation contributes at one moment.
#[derive(Clone, Debug, PartialEq)]
pub struct AnimationSample {
    /// Property values to apply over the computed style; empty outside
    /// the active interval without a fill.
    pub values: Vec<(PropertyId, Value)>,
    /// Whether the last iteration has ended.
    pub finished: bool,
}

impl Animation {
    pub(super) fn new(spec: AnimationSpec, keyframes: ResolvedKeyframes) -> Self {
        Self { spec, keyframes }
    }

    /// The spec.
    #[must_use]
    pub fn spec(&self) -> &AnimationSpec {
        &self.spec
    }

    /// The sample `elapsed_ms` after the animation started. A paused
    /// animation is the caller's: stop advancing `elapsed_ms`.
    #[must_use]
    pub fn sample(&self, elapsed_ms: f64) -> AnimationSample {
        let active = elapsed_ms - self.spec.delay_ms;
        let fill = self.spec.fill_mode;
        if active < 0.0 {
            let values = if matches!(fill, FillMode::Backwards | FillMode::Both) {
                self.values_at(self.directed(0, 0.0))
            } else {
                Vec::new()
            };
            return AnimationSample {
                values,
                finished: false,
            };
        }
        let duration = self.spec.duration_ms;
        let count = match self.spec.iteration_count {
            IterationCount::Finite(n) => n,
            IterationCount::Infinite => f64::INFINITY,
        };
        let total = duration * count;
        if duration <= 0.0 || active >= total {
            let values = if matches!(fill, FillMode::Forwards | FillMode::Both) {
                self.values_at(self.end_position(count))
            } else {
                Vec::new()
            };
            return AnimationSample {
                values,
                finished: true,
            };
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // bounded by count
        let iteration = (active / duration).floor() as u64;
        let fraction = (active / duration).fract();
        AnimationSample {
            values: self.values_at(self.directed(iteration, fraction)),
            finished: false,
        }
    }

    /// Where a finite animation rests when it ends.
    fn end_position(&self, count: f64) -> f64 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // finite, non-negative
        let last = (count.ceil() as u64).saturating_sub(1);
        let fraction = if count.fract() == 0.0 {
            1.0
        } else {
            count.fract()
        };
        self.directed(last, fraction)
    }

    /// The iteration-progress with the direction applied.
    fn directed(&self, iteration: u64, fraction: f64) -> f64 {
        let reversed = match self.spec.direction {
            Direction::Normal => false,
            Direction::Reverse => true,
            Direction::Alternate => iteration % 2 == 1,
            Direction::AlternateReverse => iteration.is_multiple_of(2),
        };
        if reversed { 1.0 - fraction } else { fraction }
    }

    /// Every animated property interpolated at `progress`.
    fn values_at(&self, progress: f64) -> Vec<(PropertyId, Value)> {
        let stops = &self.keyframes.stops;
        let mut ids: Vec<PropertyId> = stops
            .iter()
            .flat_map(|s| s.values.iter().map(|(id, _)| *id))
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids.into_iter()
            .filter_map(|id| self.value_at(id, progress).map(|v| (id, v)))
            .collect()
    }

    fn value_at(&self, id: PropertyId, progress: f64) -> Option<Value> {
        let with = |s: &&Stop| s.values.iter().any(|(i, _)| *i == id);
        let before = self
            .keyframes
            .stops
            .iter()
            .rev()
            .filter(with)
            .find(|s| s.offset <= progress)?;
        let after = self
            .keyframes
            .stops
            .iter()
            .filter(with)
            .find(|s| s.offset >= progress)?;
        let value = |s: &Stop| {
            s.values
                .iter()
                .find(|(i, _)| *i == id)
                .map(|(_, v)| v.clone())
        };
        let (from, to) = (value(before)?, value(after)?);
        if after.offset <= before.offset {
            return Some(from);
        }
        let t = (progress - before.offset) / (after.offset - before.offset);
        let eased = before.timing.unwrap_or(self.spec.timing).evaluate(t);
        Some(interpolate(&from, &to, eased).unwrap_or(if eased < 0.5 { from } else { to }))
    }
}
