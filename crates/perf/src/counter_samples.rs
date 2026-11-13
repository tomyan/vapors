//! A workload's hardware counts per kept frame, whole and per phase
//! (`--counters`), as `instruction_samples.rs` keeps instructions.

use std::collections::BTreeMap;

use crate::hardware_counters::Reading;

/// Hardware counts per kept frame; a phase a frame did not enter has
/// zeros for it.
#[derive(Debug, Default)]
pub struct CounterSamples {
    /// `App::frame`'s counts.
    pub frame: Vec<Reading>,
    /// Each phase's counts per frame, by phase name.
    pub phases: BTreeMap<String, Vec<Reading>>,
}

impl CounterSamples {
    /// Adds one frame's counts; phases are aligned by name.
    pub(crate) fn push(&mut self, frame: Reading, phases: BTreeMap<String, Reading>) {
        let earlier = self.frame.len();
        for (phase, counts) in phases {
            let kept = self.phases.entry(phase).or_default();
            kept.resize(earlier, Reading::default());
            kept.push(counts);
        }
        self.frame.push(frame);
        for kept in self.phases.values_mut() {
            kept.resize(self.frame.len(), Reading::default());
        }
    }

    /// The frames at `first`, `first + step` and on, with their phases.
    #[must_use]
    pub(crate) fn every_nth(&self, first: usize, step: usize) -> Self {
        let pick = |kept: &[Reading]| kept.iter().copied().skip(first).step_by(step).collect();
        Self {
            frame: pick(&self.frame),
            phases: self
                .phases
                .iter()
                .map(|(phase, kept)| (phase.clone(), pick(kept)))
                .collect(),
        }
    }
}
