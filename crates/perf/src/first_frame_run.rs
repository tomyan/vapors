//! `--first-frame <rows>`: property 7's measurement of one page in this
//! process, as a user's launch is (review of #428): the Mount page
//! mounted, `App::start` to the GPU's idle, then frame 0 to submit and
//! to the GPU's idle, with Mesa's shader cache named and its state
//! printed (cold or warm), so `scripts/first-frame-runs.sh` can run
//! cold and warm processes and `scripts/first-frame-table.sh` can
//! table them. Times are taken on release, instructions on the judged
//! profile (`build_profile.rs`); the header says which this is.

use std::time::Duration;

use vapors_gpu_harness::Gpu;

use crate::build_profile::{self, BUILT, JUDGED};
use crate::first_frame_report::report;
use crate::instructions_check::checked_counter;
use crate::mount::Mount;
use crate::run::Workload;
use crate::sampler::Sampler;
use crate::scene_judge::load_average;
use crate::scene_run::REVISION;
use crate::shader_cache::shader_cache;
use crate::spans::PhaseClock;

/// Runs the page of `rows` rows on the GPU `open` makes and prints the
/// report.
///
/// # Errors
/// Another profile than release or judged, Mesa's default shader cache,
/// a mount that failed, `App::start` or frame 0 refused, or a GPU wait
/// past its deadline.
pub fn run(open: impl FnOnce() -> Gpu, rows: usize) -> Result<(), String> {
    let profile = profile(BUILT)?;
    let cache = shader_cache(|name| std::env::var(name).ok())?;
    let gpu = open();
    println!(
        "Revision: {REVISION}\nProfile: {profile}\nShader cache: {}",
        cache.describe()
    );
    println!("Load average at the start: {}", load());
    let mut workload = Mount::new(rows, checked_counter()?);
    workload.ready()?;
    let mut sampler = Sampler::with_clock(&gpu, PhaseClock::with_instructions(checked_counter()?));
    let first = sampler.first_frame(&mut workload)?;
    println!("\n{}", report(rows, workload.mounted[0], &first));
    println!("Load average at the end: {}", load());
    Ok(())
}

/// The header for `built`: release for times, judged for instructions.
///
/// # Errors
/// Any other profile, naming the two builds.
pub fn profile(built: &str) -> Result<String, String> {
    match built {
        "release" => Ok(build_profile::shipped_header(built)),
        JUDGED => Ok(build_profile::header_for(built)),
        other => Err(format!(
            "a --first-frame run built with the {other} profile; its times are taken on release \
             and its instructions on {JUDGED}; fix: cargo build --release -p vapors-perf, or \
             cargo build --profile {JUDGED} -p vapors-perf"
        )),
    }
}

/// The 1-minute load average, two decimals, or why it is unknown.
fn load() -> String {
    load_average().map_or_else(
        |why| format!("unknown ({why})"),
        |load| format!("{load:.2}"),
    )
}

/// A duration in µs, one decimal, as every table here writes it.
#[must_use]
pub fn micros(time: Duration) -> String {
    format!("{:.1}", time.as_secs_f64() * 1e6)
}

#[cfg(test)]
#[path = "first_frame_run_tests.rs"]
mod tests;
