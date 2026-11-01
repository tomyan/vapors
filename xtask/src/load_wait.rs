//! Before `cargo xtask ci` and `cargo xtask gate` start: wait while the
//! machine's one-minute load is over `[load] threshold`, saying so every
//! [`SAY_EVERY`], and go on at `[load] deadline_minutes` saying that it
//! did. Several worktrees share the dev machine; a run started into a
//! saturated one has timed out on its deadlines rather than failed on
//! its code. An unreadable load is refused, never taken as low. A `ci`
//! run inside a gate does not wait: the gate already did (`main.rs`,
//! the in-gate branch of `ci_sharing_the_gate_lock`).

use std::time::{Duration, Instant};

use crate::config::Config;

/// How often the wait says the load it is waiting on.
pub const SAY_EVERY: Duration = Duration::from_secs(30);
/// The deadline when `[load] deadline_minutes` is unset: the 10 minutes
/// agents were asked to wait by hand before this (the factory's build
/// etiquette, 2026-10-26), not a measurement.
pub const DEFAULT_DEADLINE: Duration = Duration::from_mins(10);
/// Set by GitHub Actions on its runners.
pub const GITHUB_ENV: &str = "GITHUB_ACTIONS";

/// When to wait and for how long.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    /// Wait while the one-minute load is over this.
    pub threshold: f64,
    /// Then go on, saying so.
    pub deadline: Duration,
}

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

/// `[load] threshold` and `[load] deadline_minutes`; the threshold
/// defaults to `cores`, the deadline to [`DEFAULT_DEADLINE`].
///
/// # Errors
///
/// A value that is not a positive number, naming the key and the fix.
pub fn settings(config: &Config, cores: usize) -> Result<Settings, String> {
    let threshold = match config.get("load.threshold") {
        None => f64::from(u32::try_from(cores).unwrap_or(u32::MAX)),
        Some(text) => positive("threshold", text)?,
    };
    let deadline = match config.get("load.deadline_minutes") {
        None => DEFAULT_DEADLINE,
        Some(text) => Duration::from_secs_f64(positive("deadline_minutes", text)? * 60.0),
    };
    Ok(Settings {
        threshold,
        deadline,
    })
}

fn positive(key: &str, text: &str) -> Result<f64, String> {
    match text.parse::<f64>() {
        Ok(value) if value.is_finite() && value > 0.0 => Ok(value),
        _ => Err(format!(
            "[load] {key} is `{text}`, not a positive number; fix: .config/factory.toml"
        )),
    }
}

/// Wait while `load` reads over the threshold, calling `say` with each
/// line; `github` skips it.
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
    // Relaxed: GitHub's runners are single-tenant, so nothing else loads
    // them; the wait guards the shared dev machine only.
    if github {
        return Ok(Outcome::Skipped(
            "on GitHub Actions, whose runners are single-tenant",
        ));
    }
    let started = clock.now();
    loop {
        let now = load()?;
        let waited = clock.now().saturating_duration_since(started);
        if now <= settings.threshold {
            return Ok(Outcome::Clear);
        }
        // Relaxed guard: at the deadline the run goes on, loudly, rather
        // than failing. The wait protects other agents' build speed only,
        // not this run's result, and a failure here would have callers
        // retry in a loop that adds load. Retired when runs queue for
        // CPUs through a scheduler instead of this wait.
        if waited >= settings.deadline {
            say(&past_deadline(now, settings, waited));
            return Ok(Outcome::PastDeadline);
        }
        say(&waiting(now, settings, waited));
        clock.sleep(SAY_EVERY.min(settings.deadline.saturating_sub(waited)));
    }
}

fn waiting(load: f64, settings: Settings, waited: Duration) -> String {
    format!(
        "load: {load:.2} is over {} ([load] threshold); waiting, {} s so far, until {} s",
        settings.threshold,
        waited.as_secs(),
        settings.deadline.as_secs()
    )
}

fn past_deadline(load: f64, settings: Settings, waited: Duration) -> String {
    format!(
        "load: STILL {load:.2}, over {} ([load] threshold), after waiting {} s; going on \
         anyway at the deadline ([load] deadline_minutes), so timings in this run may fail \
         from the load, not the code",
        settings.threshold,
        waited.as_secs()
    )
}

/// The wait before `command` on this machine: `/proc/loadavg`, the
/// settings in `.config/factory.toml`, the core count, and whether this
/// is GitHub Actions.
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
    let mut load = || {
        crate::load::now()?
            .parse::<f64>()
            .map_err(|e| format!("/proc/loadavg: {e}"))
    };
    let mut say = |line: &str| println!("{command}: {line}");
    let outcome = wait(settings, github, &mut load, &RealClock, &mut say)?;
    if let Outcome::Skipped(why) = outcome {
        say(&format!("load: not waited for, {why}"));
    }
    Ok(outcome)
}

#[cfg(test)]
#[path = "load_wait_tests.rs"]
mod tests;
