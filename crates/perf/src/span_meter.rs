//! What each phase retired on a counter, summed by phase name between
//! the spans' enters and exits as `PhaseClock` sums their times. The
//! counter reads `N` values at once: the instruction counter one
//! (`--instructions`).

use std::collections::{BTreeMap, HashMap};

use crate::instructions::InstructionCounter;

/// A counter of `N` values that only rise.
pub(crate) trait Counting<const N: usize> {
    /// Every value now.
    fn read_all(&mut self) -> Result<[u64; N], String>;
}

impl Counting<1> for InstructionCounter {
    fn read_all(&mut self) -> Result<[u64; 1], String> {
        self.read().map(|count| [count])
    }
}

/// A counter and what it measured since the last take.
pub(crate) struct Meter<C, const N: usize> {
    counter: C,
    entered: HashMap<u64, [u64; N]>,
    totals: BTreeMap<String, [u64; N]>,
    failure: Option<String>,
}

impl<C: Counting<N>, const N: usize> Meter<C, N> {
    pub(crate) fn new(counter: C) -> Self {
        Self {
            counter,
            entered: HashMap::new(),
            totals: BTreeMap::new(),
            failure: None,
        }
    }

    /// The counter now.
    pub(crate) fn reading(&mut self) -> Result<[u64; N], String> {
        self.counter.read_all()
    }

    pub(crate) fn enter(&mut self, span: u64) {
        if let Some(now) = self.read_or_fail() {
            self.entered.insert(span, now);
        }
    }

    pub(crate) fn exit(&mut self, span: u64, phase: &str) {
        let (Some(now), Some(entered)) = (self.read_or_fail(), self.entered.remove(&span)) else {
            return;
        };
        let total = self.totals.entry(phase.to_owned()).or_insert([0; N]);
        for ((sum, now), entered) in total.iter_mut().zip(now).zip(entered) {
            *sum += now - entered;
        }
    }

    /// Each phase's values since the last take.
    ///
    /// # Errors
    /// The first counter read that failed since the last take.
    pub(crate) fn take(&mut self) -> Result<BTreeMap<String, [u64; N]>, String> {
        let totals = std::mem::take(&mut self.totals);
        self.failure.take().map_or(Ok(totals), Err)
    }

    fn read_or_fail(&mut self) -> Option<[u64; N]> {
        match self.counter.read_all() {
            Ok(now) => Some(now),
            Err(error) => {
                self.failure.get_or_insert(error);
                None
            }
        }
    }
}

#[cfg(test)]
#[path = "span_meter_tests.rs"]
mod tests;
