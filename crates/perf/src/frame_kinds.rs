//! A full run's report per kind of frame: a workload whose change
//! alternates kinds of frame (`Workload::frame_kinds`; an off-path
//! Insert inserts on one frame and removes on the next) gets a section
//! per kind, as one median over alternating frames is the cheaper
//! kind's slowest frame (`--scaling` splits its kinds the same way,
//! `scaling_kinds.rs`). Frames are named by their index from frame 0,
//! warmup counted: the sampler records the warmup in `Samples::warmup`,
//! so no caller passes it.

use std::collections::BTreeMap;

use crate::instructions_check::samples_counted;
use crate::report::{instructions_section, section};
use crate::run::Samples;
use crate::scaling_kinds::MIN_FRAMES_PER_KIND;

impl Samples {
    /// The kept frames of kind `kind` of `step` kinds (frame `i` is of
    /// kind `i % step`, and the first kept frame is frame `warmup`), with
    /// their GPU times, which the sampler keeps in frame order though it
    /// reads each back a frame late; the counts are the last frame's.
    #[must_use]
    pub fn of_kind(&self, kind: usize, step: usize) -> Self {
        let first = (kind + step - self.warmup % step) % step;
        let pick = |times: &[_]| times.iter().copied().skip(first).step_by(step).collect();
        let phases: BTreeMap<_, _> = self
            .phases
            .iter()
            .map(|(phase, times)| (phase.clone(), pick(times)))
            .collect();
        Self {
            frame: pick(&self.frame),
            phases,
            gpu: self.gpu.as_deref().map(pick),
            gpu_time: self.gpu_time.as_deref().map(pick),
            gpu_resolution: self.gpu_resolution,
            instructions: self.instructions.as_ref().map(|i| i.every_nth(first, step)),
            counts: self.counts,
            warmup: self.warmup,
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
            one_section(&title, &samples.of_kind(at, kinds.len()), instructions)
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
