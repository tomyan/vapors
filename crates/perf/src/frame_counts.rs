//! What the phase clock's counter reads over one frame (`sampler.rs`):
//! the frame's instructions, whole and per phase, when the clock was
//! made with a counter.

use std::collections::BTreeMap;

use vapors_renderer::WRITE_PHASE;

use crate::instruction_samples::InstructionSamples;
use crate::run::Samples;
use crate::spans::PhaseClock;

/// The counter at the start of a frame.
pub(crate) struct Start {
    instructions: Option<u64>,
}

/// One frame's counts, whole and per phase.
#[derive(Default)]
pub(crate) struct Counted {
    instructions: Option<(u64, BTreeMap<String, u64>)>,
}

impl Start {
    /// The counter now.
    pub(crate) fn now(clock: &PhaseClock) -> Result<Self, String> {
        Ok(Self {
            instructions: clock.instructions_now()?,
        })
    }

    /// The frame's counts: the rise since the start, with each phase's
    /// counts taken from `clock`, a `write` phase of zero added as the
    /// times' (a frame that writes no buffer enters no `write` span).
    pub(crate) fn since(self, clock: &PhaseClock) -> Result<Counted, String> {
        let instructions = self.instructions.zip(clock.instructions_now()?);
        let phases = take(clock)?;
        Ok(Counted {
            instructions: instructions
                .map(|(before, after)| after - before)
                .zip(phases.map(with_write)),
        })
    }
}

/// Each phase's counts since the last take, discarded or kept.
pub(crate) fn take(clock: &PhaseClock) -> Result<Option<BTreeMap<String, u64>>, String> {
    clock.take_instructions()
}

fn with_write<T: Default>(mut phases: BTreeMap<String, T>) -> BTreeMap<String, T> {
    phases.entry(WRITE_PHASE.into()).or_default();
    phases
}

impl Samples {
    /// Keeps one timed frame's counts.
    pub(crate) fn push_counts(&mut self, counted: Counted) {
        if let Some((frame, phases)) = counted.instructions {
            let kept = self
                .instructions
                .get_or_insert_with(InstructionSamples::default);
            kept.push(frame, phases);
        }
    }
}
