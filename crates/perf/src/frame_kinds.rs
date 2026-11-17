//! A full run's report per kind of frame: a workload whose change
//! alternates kinds of frame (`Workload::frame_kinds`; an off-path
//! Insert inserts on one frame and removes on the next) gets a section
//! per kind, as one median over alternating frames is the cheaper
//! kind's slowest frame (`--scaling` splits its kinds the same way,
//! `scaling_kinds.rs`). Frames are named by their index from frame 0,
//! warmup counted: the sampler records the warmup in `Samples::warmup`,
//! so no caller passes it.

use std::collections::BTreeMap;

use crate::args::Measure;
use crate::counters_report::counters_section;
use crate::instructions_check::{phases_counted, samples_counted};
use crate::report::{instructions_section, section};
use crate::run::Samples;
use crate::scaling_kinds::MIN_FRAMES_PER_KIND;

impl Samples {
    /// The kept frames of kind `kind` of `step` kinds (frame `i` is of
    /// kind `i % step`, and the first kept frame is frame `warmup`), with
    /// their GPU times, which the sampler keeps in frame order though it
    /// reads each back a frame late; the counts are the last frame's.
    ///
    /// # Errors
    /// A column without one entry per kept frame, naming it: its entries
    /// would be read as the other kind's.
    pub fn of_kind(&self, kind: usize, step: usize) -> Result<Self, String> {
        self.columns_kept_whole()?;
        let first = self.first_of_kind(kind, step);
        let pick = |times: &[_]| times.iter().copied().skip(first).step_by(step).collect();
        let phases: BTreeMap<_, _> = self
            .phases
            .iter()
            .map(|(phase, times)| (phase.clone(), pick(times)))
            .collect();
        Ok(Self {
            frame: pick(&self.frame),
            phases,
            gpu: self.gpu.as_deref().map(pick),
            gpu_time: self.gpu_time.as_deref().map(pick),
            gpu_resolution: self.gpu_resolution,
            instructions: self.instructions.as_ref().map(|i| i.every_nth(first, step)),
            counters: self.counters.as_ref().map(|c| c.every_nth(first, step)),
            counts: self.counts,
            warmup: self.warmup,
        })
    }

    /// The first kept frame of kind `kind` of `step` kinds: frame `i` is
    /// of kind `i % step`, and kept frame `j` is frame `warmup + j`. The
    /// one answer `--scaling` and the full run's sections both read.
    #[must_use]
    pub fn first_of_kind(&self, kind: usize, step: usize) -> usize {
        (kind + step - self.warmup % step) % step
    }

    fn columns_kept_whole(&self) -> Result<(), String> {
        let kept = self.frame.len();
        match self
            .column_lengths()
            .into_iter()
            .find(|(_, len)| *len != kept)
        {
            None => Ok(()),
            Some((column, len)) => Err(format!(
                "{column} has {len} entries for {kept} kept frames; the sampler keeps one per \
                 frame (crates/perf/src/sampler.rs), and a column out of step would be read as \
                 the other kind's"
            )),
        }
    }

    /// Each column a kept frame has an entry in, with its length.
    fn column_lengths(&self) -> Vec<(&str, usize)> {
        let mut lengths: Vec<_> = self
            .phases
            .iter()
            .map(|(phase, times)| (phase.as_str(), times.len()))
            .collect();
        let optional = [
            ("GPU span", self.gpu.as_ref().map(Vec::len)),
            ("GPU time", self.gpu_time.as_ref().map(Vec::len)),
            (
                "instructions",
                self.instructions.as_ref().map(|i| i.frame.len()),
            ),
            (
                "hardware counts",
                self.counters.as_ref().map(|c| c.frame.len()),
            ),
        ];
        lengths.extend(
            optional
                .into_iter()
                .filter_map(|(name, len)| Some((name, len?))),
        );
        lengths
    }
}

/// The report of workload `name`'s `samples`: one section, or one per
/// kind of frame in `kinds`, titled `<name>, <kind> frames`.
///
/// # Errors
/// Fewer than [`MIN_FRAMES_PER_KIND`] kept frames of a kind, naming the
/// fix; or, counting instructions or hardware events, a phase without
/// a count.
pub fn sections(
    name: &str,
    samples: &Samples,
    kinds: &[&str],
    measure: Measure,
) -> Result<Vec<String>, String> {
    if kinds.len() < 2 {
        return Ok(vec![one_section(name, samples, measure)?]);
    }
    enough_frames(name, samples.frame.len(), kinds)?;
    kinds
        .iter()
        .enumerate()
        .map(|(at, kind)| {
            let title = format!("{name}, {kind} frames");
            one_section(&title, &samples.of_kind(at, kinds.len())?, measure)
        })
        .collect()
}

fn one_section(name: &str, samples: &Samples, measure: Measure) -> Result<String, String> {
    match measure {
        Measure::Times => Ok(section(name, samples)),
        Measure::Instructions => {
            samples_counted(name, samples)?;
            instructions_section(name, samples)
        }
        Measure::Counters => {
            let counted = samples.counters.as_ref().map(|kept| &kept.phases);
            phases_counted(&samples.phases, counted.unwrap_or(&BTreeMap::new()))
                .map_err(|e| format!("{name}: {e}"))?;
            counters_section(name, samples)
        }
    }
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
