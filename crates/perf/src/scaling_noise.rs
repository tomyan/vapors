//! The growth the scaling judge sees on a page that does no work that
//! depends on its size (#505), which `scaling::MIN_GROWTH_INSTRUCTIONS`
//! must stay above or the nightly fails with no change to blame.
//!
//! `FlexText`'s draw phase, at b4508baf (judged profile, `vapors-perf 60
//! 10 --scaling FlexText`, twelve runs), counted 4054, 4067, 4162, 4241,
//! 4412, 4421, 4782, 4887, 4933, 5008, 5146 and 5196 more instructions
//! at 3000 or 10,000 rows than at 1000 (the larger of the two). The
//! growth is a step between 1000 and 3000 rows, flat from 3000 to
//! 10,000 (0.0 to 0.1 per extra row), and present at 6a6dc79d, before
//! the clip table (3290 at 5699e9ed, where the nightly last passed):
//! not per-row work, and not the clip-table commits (#494, #495). One
//! text batch of 13 items is drawn at every size. Returning early from
//! `draw` at the batches, the painter or the whole of `record` left
//! 2400, 2460 and 1400 of it, so it is spread over code that does
//! nothing size dependent: likely heap state once 3000 rows are held.
//! Equalising the edited row's label length changed nothing.
//! Real per-row work is far above it: one instruction per row is 9000
//! over the 9000 extra rows.

/// The largest growth of those twelve runs, in instructions.
pub const MAX_OBSERVED_GROWTH: u64 = 5196;

/// The margin the floor keeps over [`MAX_OBSERVED_GROWTH`], in percent.
pub const MARGIN_PERCENT: u64 = 25;

/// The instructions of growth one instruction per extra row makes from
/// 1000 to 10,000 rows: the floor stays under it, or a phase that gains
/// an instruction per row passes.
pub const ONE_PER_ROW: u64 = 9000;

#[cfg(test)]
#[path = "scaling_noise_tests.rs"]
mod tests;
