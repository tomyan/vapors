//! A workload's instructions retired per frame, whole and per phase.

use std::collections::BTreeMap;

/// Instruction counts per timed frame; a phase a frame did not enter has
/// 0 for it.
#[derive(Debug, Default)]
pub struct InstructionSamples {
    /// `App::frame`'s instructions.
    pub frame: Vec<u64>,
    /// Each phase's instructions per frame, by phase name.
    pub phases: BTreeMap<String, Vec<u64>>,
}

impl InstructionSamples {
    /// Adds one frame's counts; phases are aligned by name.
    pub(crate) fn push(&mut self, frame: u64, phases: BTreeMap<String, u64>) {
        let earlier = self.frame.len();
        for (phase, count) in phases {
            let counts = self.phases.entry(phase).or_default();
            counts.resize(earlier, 0);
            counts.push(count);
        }
        self.frame.push(frame);
        for counts in self.phases.values_mut() {
            counts.resize(self.frame.len(), 0);
        }
    }

    /// The frames at `first`, `first + step`, `first + 2 step` and on,
    /// with their phases.
    #[must_use]
    pub(crate) fn every_nth(&self, first: usize, step: usize) -> Self {
        let pick = |counts: &[u64]| counts.iter().copied().skip(first).step_by(step).collect();
        Self {
            frame: pick(&self.frame),
            phases: self
                .phases
                .iter()
                .map(|(phase, counts)| (phase.clone(), pick(counts)))
                .collect(),
        }
    }
}

#[cfg(test)]
#[path = "instruction_samples_tests.rs"]
mod tests;
