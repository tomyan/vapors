//! What the phase clock's counters read over one frame (`sampler.rs`):
//! the frame's instructions or hardware counts, whole and per phase,
//! when the clock was made with them.

use std::collections::BTreeMap;

use vapors_app::SURFACE_PHASE;
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
        // the counters first and instructions last, so an instructions
        // run's window holds no read of absent counters
        let counters = clock.counters_now()?;
        Ok(Self {
            instructions: clock.instructions_now()?,
            counters,
        })
    }

    /// The frame's rise since the start, read when the app's frame
    /// returns; its phases are taken later (`Rise::with_phases`).
    pub(crate) fn since(self, clock: &PhaseClock) -> Result<Rise, String> {
        let instructions = self.instructions.zip(clock.instructions_now()?);
        let counters = self.counters.zip(clock.counters_now()?);
        Ok(Rise {
            instructions: instructions.map(|(before, after)| after - before),
            counters: counters.map(|(before, after)| rise(before, after)),
        })
    }
}

/// The counters' rise over the app's frame.
pub(crate) struct Rise {
    instructions: Option<u64>,
    counters: Option<Reading>,
}

impl Rise {
    /// The frame's counts: the rise, with each phase's counts as taken
    /// (`take`), a `write` phase of zero added as the times' (a frame
    /// that writes no buffer enters no `write` span).
    pub(crate) fn with_phases(self, (phases, phase_counters): Phases) -> Counted {
        Counted {
            instructions: self.instructions.zip(phases.map(with_maybe_entered)),
            counters: self.counters.zip(phase_counters.map(with_maybe_entered)),
        }
    }
}

impl Counted {
    /// The frame's instructions, whole and per phase, with a counter.
    pub(crate) fn into_instructions(self) -> Option<(u64, BTreeMap<String, u64>)> {
        self.instructions
    }
}

/// Each phase's counts since the last take, discarded or kept.
pub(crate) fn take(clock: &PhaseClock) -> Result<Phases, String> {
    Ok((clock.take_instructions()?, clock.take_counters()?))
}

pub(crate) type Phases = (
    Option<BTreeMap<String, u64>>,
    Option<BTreeMap<String, Reading>>,
);

/// The phases a frame may enter none of: `write` (nothing uploaded) and
/// `surface` (no `gpu-surface` on the page). Every frame records them,
/// as 0 when not entered, so each run has a value for every phase.
pub(crate) const MAYBE_ENTERED: [&str; 2] = [WRITE_PHASE, SURFACE_PHASE];

/// `phases` with each of [`MAYBE_ENTERED`] it lacks, as 0.
pub(crate) fn with_maybe_entered<T: Default>(
    mut phases: BTreeMap<String, T>,
) -> BTreeMap<String, T> {
    for phase in MAYBE_ENTERED {
        phases.entry(phase.into()).or_default();
    }
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

#[cfg(test)]
#[path = "frame_counts_tests.rs"]
mod tests;
