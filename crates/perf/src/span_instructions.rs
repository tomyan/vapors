//! The instructions each phase retired, summed by phase name between the
//! spans' enters and exits as `PhaseClock` sums their times.

use std::collections::{BTreeMap, HashMap};

use crate::instructions::InstructionCounter;

/// A counter and what it measured since the last take.
pub(crate) struct InstructionMeter {
    counter: InstructionCounter,
    entered: HashMap<u64, u64>,
    totals: BTreeMap<String, u64>,
    failure: Option<String>,
}

impl InstructionMeter {
    pub(crate) fn new(counter: InstructionCounter) -> Self {
        Self {
            counter,
            entered: HashMap::new(),
            totals: BTreeMap::new(),
            failure: None,
        }
    }

    /// The counter now.
    pub(crate) fn reading(&mut self) -> Result<u64, String> {
        self.counter.read()
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
        *self.totals.entry(phase.to_owned()).or_default() += now - entered;
    }

    /// Each phase's instructions since the last take.
    ///
    /// # Errors
    /// The first counter read that failed since the last take.
    pub(crate) fn take(&mut self) -> Result<BTreeMap<String, u64>, String> {
        let totals = std::mem::take(&mut self.totals);
        self.failure.take().map_or(Ok(totals), Err)
    }

    fn read_or_fail(&mut self) -> Option<u64> {
        match self.counter.read() {
            Ok(now) => Some(now),
            Err(error) => {
                self.failure.get_or_insert(error);
                None
            }
        }
    }
}
