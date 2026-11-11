//! How much one command encoder of a [`FrameEncoder`](crate::FrameEncoder)
//! holds before the frame flushes it and goes on in a new one.

use std::cell::Cell;

/// The most render passes and draws one command encoder takes; the
/// frame's encoder flushes before a draw, or a new pass, that would pass
/// its half. Neither may be 0 ([`FrameEncoder::with_budget`](crate::FrameEncoder::with_budget)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EncoderBudget {
    /// Render passes per command encoder.
    pub passes: u32,
    /// Draws per command encoder.
    pub draws: u32,
}

impl Default for EncoderBudget {
    /// Passes: measured on ANV (Intel UHD 630, Mesa) with
    /// `cargo run --release -p vapors-perf --example encoder_budget`
    /// (2026-11-11): `CommandEncoder::finish` failed with Out of Memory
    /// at 8200 render passes and passed at 8000. 4096 is half of 8000.
    ///
    /// Draws: no limit was found. One pass of 729,000 draws passed, its
    /// finish and wait growing in step with the draws; 100,000 is a
    /// margin chosen below that, not a measured limit. Dawn (the web),
    /// Metal and lavapipe are not measured for either number.
    fn default() -> Self {
        Self {
            passes: 4096,
            draws: 100_000,
        }
    }
}

/// What the current command encoder holds, against its budget.
pub(crate) struct Spent {
    budget: EncoderBudget,
    passes: Cell<u32>,
    draws: Cell<u32>,
}

impl Spent {
    pub(crate) fn new(budget: EncoderBudget) -> Self {
        Self {
            budget,
            passes: Cell::new(0),
            draws: Cell::new(0),
        }
    }

    /// Whether one more pass would pass the budget.
    pub(crate) fn passes_full(&self) -> bool {
        self.passes.get() >= self.budget.passes
    }

    /// Whether one more draw would pass the budget.
    pub(crate) fn draws_full(&self) -> bool {
        self.draws.get() >= self.budget.draws
    }

    pub(crate) fn pass(&self) {
        self.passes.set(self.passes.get() + 1);
    }

    pub(crate) fn draw(&self) {
        self.draws.set(self.draws.get() + 1);
    }

    /// A new command encoder: nothing spent.
    pub(crate) fn reset(&self) {
        self.passes.set(0);
        self.draws.set(0);
    }
}
