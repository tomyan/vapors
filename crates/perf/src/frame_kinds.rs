//! A full run's report per kind of frame: a workload whose change
//! alternates kinds of frame (`Workload::frame_kinds`; an off-path
//! Insert inserts on one frame and removes on the next) gets a section
//! per kind, as one median over alternating frames is the cheaper
//! kind's slowest frame (`--scaling` splits its kinds the same way,
//! `scaling_kinds.rs`).

use std::collections::BTreeMap;

use crate::instructions_check::samples_counted;
use crate::report::{PHASES, instructions_section, section, times_section};
use crate::run::Samples;
use crate::scaling_kinds::MIN_FRAMES_PER_KIND;

/// Said under a kind's section: why it has no GPU rows.
pub const NO_GPU_ROWS: &str = "GPU rows left out: the app's GPU time is read back a frame late, \
                               so a frame's index does not name its kind.";

impl Samples {
    /// The kept frames of kind `kind` of `step` kinds, whose first kept
    /// frame is frame `warmup` (frame `i` is of kind `i % step`), with
    /// no GPU times; the counts are the last frame's.
    #[must_use]
    pub fn of_kind(&self, kind: usize, step: usize, warmup: usize) -> Self {
        let first = (kind + step - warmup % step) % step;
        let pick = |times: &[_]| times.iter().copied().skip(first).step_by(step).collect();
        let phases: BTreeMap<_, _> = self
            .phases
            .iter()
            .map(|(phase, times)| (phase.clone(), pick(times)))
            .collect();
        Self {
            frame: pick(&self.frame),
            phases,
            instructions: self.instructions.as_ref().map(|i| i.every_nth(first, step)),
            counts: self.counts,
            ..Self::default()
        }
    }
}

/// The report of workload `name`'s `samples`: one section, or one per
/// kind of frame in `kinds`, titled `<name>, <kind> frames`.
///
/// # Errors
/// Fewer than [`MIN_FRAMES_PER_KIND`] kept frames of a kind, naming the
/// fix; or, counting `instructions`, a phase without a count.
pub fn sections(
    name: &str,
    samples: &Samples,
    kinds: &[&str],
    warmup: usize,
    instructions: bool,
) -> Result<Vec<String>, String> {
    if kinds.len() < 2 {
        return Ok(vec![one_section(name, samples, instructions)?]);
    }
    enough_frames(name, samples.frame.len(), kinds)?;
    kinds
        .iter()
        .enumerate()
        .map(|(at, kind)| {
            let title = format!("{name}, {kind} frames");
            kind_section(
                &title,
                &samples.of_kind(at, kinds.len(), warmup),
                instructions,
            )
        })
        .collect()
}

fn one_section(name: &str, samples: &Samples, instructions: bool) -> Result<String, String> {
    if !instructions {
        return Ok(section(name, samples));
    }
    samples_counted(name, samples)?;
    instructions_section(name, samples)
}

fn kind_section(title: &str, samples: &Samples, instructions: bool) -> Result<String, String> {
    let report = if instructions {
        one_section(title, samples, true)?
    } else {
        times_section(title, samples, &PHASES)
    };
    Ok(format!("{report}\n{NO_GPU_ROWS}\n"))
}

fn enough_frames(name: &str, runs: usize, kinds: &[&str]) -> Result<(), String> {
    let least = kinds.len() * MIN_FRAMES_PER_KIND;
    if runs >= least {
        return Ok(());
    }
    Err(format!(
        "{name}: its frames alternate {} kinds ({}) and {runs} runs leave fewer than \
         {MIN_FRAMES_PER_KIND} of each; give at least {least} runs: vapors-perf {least} 10 --off-path",
        kinds.len(),
        kinds.join(", ")
    ))
}

#[cfg(test)]
#[path = "frame_kinds_tests.rs"]
mod tests;
