//! A running transition, sampled by elapsed time.

use super::interpolate::interpolate;
use super::spec::TransitionSpec;
use crate::value::Value;

/// One property transitioning from one computed value to another.
#[derive(Clone, Debug, PartialEq)]
pub struct Transition {
    spec: TransitionSpec,
    from: Value,
    to: Value,
}

impl Transition {
    /// A transition from `from` to `to` under `spec`. To retarget a
    /// running transition, pass its current sample as `from`.
    #[must_use]
    pub fn new(spec: TransitionSpec, from: Value, to: Value) -> Self {
        Self { spec, from, to }
    }

    /// The spec.
    #[must_use]
    pub fn spec(&self) -> &TransitionSpec {
        &self.spec
    }

    /// The target value.
    #[must_use]
    pub fn target(&self) -> &Value {
        &self.to
    }

    /// The value `elapsed_ms` after the transition started, and whether
    /// it has finished.
    #[must_use]
    pub fn sample(&self, elapsed_ms: f64) -> (Value, bool) {
        let active = elapsed_ms - self.spec.delay_ms;
        if active <= 0.0 {
            return (self.from.clone(), false);
        }
        if active >= self.spec.duration_ms {
            return (self.to.clone(), true);
        }
        let eased = self.spec.timing.evaluate(active / self.spec.duration_ms);
        let value = interpolate(&self.from, &self.to, eased).unwrap_or_else(|| {
            if eased < 0.5 {
                self.from.clone()
            } else {
                self.to.clone()
            }
        });
        (value, false)
    }
}
