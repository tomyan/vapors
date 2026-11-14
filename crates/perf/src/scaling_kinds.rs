//! The kinds of frame a `--scaling` change alternates between
//! (`RowChange::frame_kinds`): enough kept frames of each, a median per
//! kind at each size, and a series per kind across sizes.

use crate::report::EVERY_PHASE;
use crate::rows::RowChange;
use crate::run::Samples;
use crate::scaling::{AtSize, Scaling};
use crate::scaling_run::{kept_counts, medians};

/// The fewest kept frames of each kind: with three, a kind's median is
/// never its fastest or its slowest frame alone.
pub(crate) const MIN_FRAMES_PER_KIND: usize = 3;

/// Whether `runs` kept frames give each kind of frame `change` makes
/// [`MIN_FRAMES_PER_KIND`].
///
/// # Errors
/// Too few, naming the change, its kinds and the command.
pub(crate) fn enough_frames(change: RowChange, runs: usize) -> Result<(), String> {
    let kinds = change.frame_kinds();
    let least = kinds.len() * MIN_FRAMES_PER_KIND;
    if runs >= least {
        return Ok(());
    }
    Err(format!(
        "scaling: {change:?} alternates {} kinds of frame ({}) and {runs} runs leave fewer than \
         {MIN_FRAMES_PER_KIND} frames of each; give at least {least} runs: vapors-perf {least} 10 \
         --scaling {change:?}",
        kinds.len(),
        kinds.join(", ")
    ))
}

/// The medians of each kind of frame in `kinds`, from `samples`: frame
/// `i` is of kind `i % kinds.len()`, and the first kept frame is frame
/// `samples.warmup`, as the sampler recorded it.
///
/// # Errors
/// As [`crate::scaling_run::at_size`], for any kind, naming it.
pub(crate) fn by_kind(
    rows: usize,
    samples: &Samples,
    kinds: &[&str],
) -> Result<Vec<AtSize>, String> {
    let kept = kept_counts(rows, samples)?;
    let step = kinds.len();
    let first_of = |kind: usize| samples.first_of_kind(kind, step);
    kinds
        .iter()
        .enumerate()
        .map(|(at, kind)| {
            medians(rows, &kept.every_nth(first_of(at), step), &EVERY_PHASE)
                .map_err(|e| format!("{kind} frames: {e}"))
        })
        .collect()
}

/// Each kind of frame's title and its series across sizes, from
/// `measured`, a list of kinds per size.
pub(crate) fn per_kind(
    change: RowChange,
    measured: Vec<Vec<AtSize>>,
) -> Result<Vec<(String, Scaling)>, String> {
    let kinds = change.frame_kinds();
    let mut by_kind: Vec<Vec<AtSize>> = kinds.iter().map(|_| Vec::new()).collect();
    for size in measured {
        if size.len() != kinds.len() {
            return Err(kinds_differ(change, size.len()));
        }
        by_kind
            .iter_mut()
            .zip(size)
            .for_each(|(sizes, at)| sizes.push(at));
    }
    kinds
        .iter()
        .zip(by_kind)
        .map(|(kind, sizes)| {
            let title = title(change, kind);
            let scaling = Scaling::of(&sizes).map_err(|e| format!("{title}: {e}"))?;
            Ok((title, scaling))
        })
        .collect()
}

fn kinds_differ(change: RowChange, measured: usize) -> String {
    format!(
        "scaling: {change:?} makes {} kinds of frame and a size was measured with {measured}; \
         by_kind must return one set of medians per entry of RowChange::frame_kinds \
         (crates/perf/src/scaling_kinds.rs)",
        change.frame_kinds().len()
    )
}

/// `change`'s name, and the kind of frame when it makes more than one.
pub(crate) fn title(change: RowChange, kind: &str) -> String {
    if change.frame_kinds().len() == 1 {
        return format!("{change:?}");
    }
    format!("{change:?}, {kind} frames")
}

#[cfg(test)]
#[path = "scaling_kinds_tests.rs"]
mod tests;
