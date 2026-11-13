//! What the phase clock's counters read over one frame (`sampler.rs`):
//! the frame's instructions or hardware counts, whole and per phase,
//! when the clock was made with them.

use std::collections::BTreeMap;

use vapors_renderer::WRITE_PHASE;

use crate::counter_samples::CounterSamples;
use crate::hardware_counters::Reading;
use crate::instruction_samples::InstructionSamples;
use crate::run::Samples;
use crate::spans::PhaseClock;

/// The counters at the start of a frame.
pub(crate) struct Start {
    instructions: Option<u64>,
    counters: Option<Reading>,
}

/// One frame's counts, whole and per phase.
#[derive(Default)]
pub(crate) struct Counted {
    instructions: Option<(u64, BTreeMap<String, u64>)>,
    counters: Option<(Reading, BTreeMap<String, Reading>)>,
}

impl Start {
    /// The counters now.
    pub(crate) fn now(clock: &PhaseClock) -> Result<Self, String> {
        Ok(Self {
            instructions: clock.instructions_now()?,
            counters: clock.counters_now()?,
        })
    }

    /// The frame's counts: the rise since the start, with each phase's
    /// counts taken from `clock`, a `write` phase of zero added as the
    /// times' (a frame that writes no buffer enters no `write` span).
    pub(crate) fn since(self, clock: &PhaseClock) -> Result<Counted, String> {
        let instructions = self.instructions.zip(clock.instructions_now()?);
        let counters = self.counters.zip(clock.counters_now()?);
        let (phases, phase_counters) = take(clock)?;
        Ok(Counted {
            instructions: instructions
                .map(|(before, after)| after - before)
                .zip(phases.map(with_write)),
            counters: counters
                .map(|(before, after)| rise(before, after))
                .zip(phase_counters.map(with_write)),
        })
    }
}

/// Each phase's counts since the last take, discarded or kept.
pub(crate) fn take(clock: &PhaseClock) -> Result<Phases, String> {
    Ok((clock.take_instructions()?, clock.take_counters()?))
}

type Phases = (
    Option<BTreeMap<String, u64>>,
    Option<BTreeMap<String, Reading>>,
);

fn with_write<T: Default>(mut phases: BTreeMap<String, T>) -> BTreeMap<String, T> {
    phases.entry(WRITE_PHASE.into()).or_default();
    phases
}

fn rise(before: Reading, after: Reading) -> Reading {
    let mut rise = Reading::default();
    for ((sum, after), before) in rise.iter_mut().zip(after).zip(before) {
        *sum = after - before;
    }
    rise
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
        if let Some((frame, phases)) = counted.counters {
            let kept = self.counters.get_or_insert_with(CounterSamples::default);
            kept.push(frame, phases);
        }
    }
}
