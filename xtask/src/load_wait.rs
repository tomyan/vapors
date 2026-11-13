//! Before `cargo xtask ci` and `cargo xtask gate` start: wait while the
//! machine's runnable tasks are over `[load] threshold`, reading them
//! every [`READ_EVERY`] and saying so every [`SAY_EVERY`], and go on at
//! `[load] deadline_minutes` saying that it did. The count of tasks
//! runnable now, not the one-minute load average: the average decays
//! for a minute or two after the work has gone (60 to 12 took 90 to
//! 120 s in gate 271, docs/plans/factory-throughput.md C3). Several
//! worktrees share the dev machine; a run started into a saturated one has timed out on its deadlines rather than failed on
//! its code. An unreadable load is refused, never taken as low, naming
//! `[load] wait = false` as the fix. A `ci` run inside a gate does not
//! wait: the gate already did (`main.rs`, the in-gate branch of
//! `ci_shared::run`). The gate waits once it holds its lock,
//! so the load it measures is what it will run beside.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use crate::config::Config;
#[path = "load_wait_settings.rs"]
mod load_settings;
pub use load_settings::{Settings, settings};

/// How often the wait says the load it is waiting on.
pub const SAY_EVERY: Duration = Duration::from_secs(30);
/// How often the wait reads the runnable tasks: a step that starts once
/// the CPUs free starts within this. Not a measurement; the read is one
/// small file, so the period costs nothing either way.
pub const READ_EVERY: Duration = Duration::from_secs(5);
/// How many readings in a row must be at or under the threshold: one
/// reading is an instant, and a build between compile bursts (a link, a
/// build script) reads low for a moment while its next jobs are about
/// to start. Three is 10 s of it, read 5 s apart. Not a measurement.
pub const CLEAR_READS: u32 = 3;
/// Set by GitHub Actions on its runners.
pub const GITHUB_ENV: &str = "GITHUB_ACTIONS";

/// How the wait ended.
#[derive(Debug, PartialEq)]
pub enum Outcome {
    /// The load was at or under the threshold.
    Clear,
    /// The deadline passed with the load still over it.
    PastDeadline,
    /// Not waited for, and why.
    Skipped(&'static str),
}

/// The machine's clock, a fake in tests.
pub trait Clock {
    /// Now.
    fn now(&self) -> Instant;
    /// Wait `period`.
    fn sleep(&self, period: Duration);
}

struct RealClock;

impl Clock for RealClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
    fn sleep(&self, period: Duration) {
        std::thread::sleep(period);
    }
}

/// Wait while `load` reads over the threshold, calling `say` with each
/// line; `github`, or `[load] wait = false`, skips it.
///
/// # Errors
///
/// The load could not be read: refused, never taken as low.
pub fn wait(
    settings: Settings,
    github: bool,
    load: &mut dyn FnMut() -> Result<f64, String>,
    clock: &dyn Clock,
    say: &mut dyn FnMut(&str),
) -> Result<Outcome, String> {
    if let Some(why) = skipped(settings, github) {
        return Ok(Outcome::Skipped(why));
    }
    let started = clock.now();
    let (mut say_at, mut under) = (Duration::ZERO, 0);
    loop {
        let now = load()?;
        let waited = clock.now().saturating_duration_since(started);
        under = if now <= settings.threshold {
            under + 1
        } else {
            0
        };
        if under >= CLEAR_READS {
            return Ok(Outcome::Clear);
        }
        // Relaxed guard: at the deadline the run goes on, loudly, rather
        // than failing. The wait protects other agents' build speed only,
        // not this run's result, and a failure here would have callers
        // retry in a loop that adds load. Retired when runs queue for
        // CPUs through a scheduler instead of this wait. Tom's choice:
        // docs/decisions/2026-11-01-load-wait-deadline.md.
        if under == 0 && waited >= settings.deadline {
            say(&past_deadline(now, settings, waited));
            return Ok(Outcome::PastDeadline);
        }
        if under == 0 && waited >= say_at {
            say(&waiting(now, settings, waited));
            say_at = waited + SAY_EVERY;
        }
        clock.sleep(READ_EVERY.min(settings.deadline.saturating_sub(waited)));
    }
}

/// Why the wait is not made, if it is not.
fn skipped(settings: Settings, github: bool) -> Option<&'static str> {
    // Relaxed: GitHub's runners are single-tenant, so nothing else loads
    // them; the wait guards the shared dev machine only.
    if github {
        return Some("on GitHub Actions, whose runners are single-tenant");
    }
    // Relaxed guard, declared: `[load] wait = false` runs without the
    // wait, for a machine with no /proc/loadavg (the planned Mac Studio),
    // said on every run. Retired when the load is read portably.
    (!settings.enabled).then_some("[load] wait = false in .config/factory.toml")
}

fn waiting(load: f64, settings: Settings, waited: Duration) -> String {
    format!(
        "load: {load} runnable tasks, over {} ([load] threshold); waiting, {} s so far, until {} s",
        settings.threshold,
        waited.as_secs(),
        settings.deadline.as_secs()
    )
}

fn past_deadline(load: f64, settings: Settings, waited: Duration) -> String {
    format!(
        "load: STILL {load} runnable tasks, over {} ([load] threshold), after waiting {} s; going on \
         anyway at the deadline ([load] deadline_minutes), so timings in this run may fail \
         from the load, not the code",
        settings.threshold,
        waited.as_secs()
    )
}

/// The wait before `command` on this machine: `/proc/loadavg`, the
/// settings in `.config/factory.toml`, the core count, and whether this
/// is GitHub Actions. A wait that went past its deadline is kept for
/// [`went_past_deadline`].
///
/// # Errors
///
/// The settings or the load could not be read, naming the fix.
pub fn before(command: &str) -> Result<Outcome, String> {
    let config = Config::load().map_err(|e| e.to_string())?;
    let cores = std::thread::available_parallelism()
        .map_err(|e| format!("the machine's core count could not be read ({e}); fix: set [load] threshold in .config/factory.toml"))?
        .get();
    let settings = settings(&config, cores)?;
    let github = std::env::var(GITHUB_ENV).is_ok_and(|v| v == "true");
    let mut load = || machine_load().map_err(|e| format!("{e}; {NO_LOAD_FIX}"));
    let mut last = String::new();
    let mut say = |line: &str| {
        println!("{command}: {line}");
        line.clone_into(&mut last);
    };
    let outcome = wait(settings, github, &mut load, &RealClock, &mut say)?;
    if let Outcome::Skipped(why) = outcome {
        say(&format!("load: not waited for, {why}"));
    }
    if outcome == Outcome::PastDeadline {
        drop(PAST.set(format!("{command}: {last}")));
    }
    Ok(outcome)
}

/// The fix when the load cannot be read.
pub const NO_LOAD_FIX: &str =
    "fix: on a machine without /proc/loadavg, set [load] wait = \"false\" in .config/factory.toml";

fn machine_load() -> Result<f64, String> {
    crate::load::runnable_now().map(f64::from)
}

static PAST: OnceLock<String> = OnceLock::new();

/// The line this run printed when it went on past the load deadline,
/// for the final summary to repeat.
pub fn went_past_deadline() -> Option<&'static str> {
    PAST.get().map(String::as_str)
}

#[cfg(test)]
#[path = "load_wait_tests.rs"]
mod tests;
