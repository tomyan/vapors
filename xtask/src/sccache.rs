//! The shared compilation cache: every build xtask starts runs rustc
//! through sccache, one cache for every worktree on the machine
//! (`~/.cache/sccache`, sccache's default), so a fresh worktree compiles
//! what another already compiled from the cache. sccache must be there
//! and its server must answer, or the build is refused naming
//! `cargo xtask setup`; a build never falls back to compiling uncached.

use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;

use serde_json::Value;

use crate::steps::Step;

/// The cache's cap: the dev box's worktrees each hold 20 to 50 GB of
/// target (2026-10-23), and the disk filled at 853 of 898 GB; one shared
/// cache of 50 GB is smaller than two worktrees' targets.
pub const CACHE_SIZE: &str = "50G";
const CACHE_SIZE_BYTES: u64 = 50 * 1024 * 1024 * 1024;

/// How long sccache may take to start its server and answer. Starting
/// and answering took 0.1 s here; 30 s is the ceiling before the server
/// is declared unable to start.
const ANSWER_DEADLINE: Duration = Duration::from_secs(30);

/// The fix every refusal names.
const FIX: &str = "fix: run `cargo xtask setup`, or `sccache --stop-server` if one is running with other settings";

/// Cache hits, misses and errors, summed over languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    /// Compilations answered from the cache.
    pub hits: u64,
    /// Compilations that ran and were stored.
    pub misses: u64,
    /// Cache errors, reads and writes that failed: on each, sccache
    /// compiled without the cache rather than failing the build.
    pub errors: u64,
}

/// Sets the cache on `command`: rustc through sccache, and no
/// incremental compilation, which sccache cannot cache.
///
/// # Errors
///
/// sccache missing, or its server unable to start or answer, or one
/// running with another local cache size.
pub fn apply(command: &mut Command) -> Result<(), String> {
    ready()?;
    command.envs(ENV);
    Ok(())
}

/// The environment `apply` sets.
pub const ENV: [(&str, &str); 4] = [
    ("RUSTC_WRAPPER", "sccache"),
    ("CARGO_INCREMENTAL", "0"),
    SERVER_ENV[0],
    SERVER_ENV[1],
];

/// The settings a server takes when it starts. `SCCACHE_IDLE_TIMEOUT=0`:
/// a server that stopped after 600 s idle and restarted would zero its
/// counters mid-run and hide cache errors.
const SERVER_ENV: [(&str, &str); 2] = [
    ("SCCACHE_CACHE_SIZE", CACHE_SIZE),
    ("SCCACHE_IDLE_TIMEOUT", "0"),
];

/// Whether sccache answered, checked once per xtask run.
pub fn ready() -> Result<(), String> {
    static READY: OnceLock<Result<(), String>> = OnceLock::new();
    READY.get_or_init(|| stats().map(|_| ())).clone()
}

/// The server's statistics now, starting it if needed.
///
/// # Errors
///
/// As [`apply`].
pub fn stats() -> Result<Stats, String> {
    let mut step = Step::new(
        "sccache",
        "sccache",
        &["--show-stats", "--stats-format", "json"],
        FIX,
    );
    // The server, if this starts it, takes these settings.
    step.env = SERVER_ENV
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect();
    step.deadline = Some(ANSWER_DEADLINE);
    let outcome = crate::command::run(&step);
    if !outcome.passed {
        return Err(format!(
            "sccache did not answer: {}; {FIX}",
            outcome.output.trim()
        ));
    }
    parse(&outcome.output)
}

/// Hits and misses from `--show-stats --stats-format json`; a local
/// cache must have the cap set here.
///
/// # Errors
///
/// Output of another shape, or a local cache of another size.
pub fn parse(output: &str) -> Result<Stats, String> {
    let json = output.find('{').map_or(output, |start| &output[start..]);
    let value: Value = serde_json::from_str(json.trim()).map_err(|e| {
        format!(
            "sccache's stats are not the JSON this reads ({e}): {}",
            output.trim()
        )
    })?;
    check_size(&value)?;
    let stats = &value["stats"];
    Ok(Stats {
        hits: count(&stats["cache_hits"]["counts"], "cache_hits")?,
        misses: count(&stats["cache_misses"]["counts"], "cache_misses")?,
        errors: count(&stats["cache_errors"]["counts"], "cache_errors")?
            + number(stats, "cache_read_errors")?
            + number(stats, "cache_write_errors")?,
    })
}

fn count(counts: &Value, name: &str) -> Result<u64, String> {
    let map = counts
        .as_object()
        .ok_or_else(|| format!("sccache's stats have no {name}.counts"))?;
    map.values()
        .map(|n| {
            n.as_u64()
                .ok_or_else(|| format!("sccache's {name} count {n} is not a whole number"))
        })
        .sum()
}

fn number(stats: &Value, name: &str) -> Result<u64, String> {
    stats[name]
        .as_u64()
        .ok_or_else(|| format!("sccache's stats have no whole-number {name}"))
}

fn check_size(value: &Value) -> Result<(), String> {
    let location = value["cache_location"].as_str().unwrap_or_default();
    if !location.starts_with("Local disk") {
        return Ok(());
    }
    match value["max_cache_size"].as_u64() {
        Some(CACHE_SIZE_BYTES) => Ok(()),
        other => Err(format!(
            "the running sccache server caps its cache at {other:?} bytes, not {CACHE_SIZE}; {FIX}"
        )),
    }
}

/// The hit rate between two readings, for the end of `cargo xtask ci`.
/// The server is shared, so builds other worktrees ran meanwhile count.
///
/// # Errors
///
/// Cache errors between the readings: those compilations ran uncached.
pub fn hit_rate(before: Stats, after: Stats) -> Result<String, String> {
    if after.hits < before.hits || after.misses < before.misses || after.errors < before.errors {
        return Err(format!(
            "sccache's counters went down ({before:?} to {after:?}): its server restarted during the run, so cache errors before the restart are not counted; fix: rerun (xtask starts the server with SCCACHE_IDLE_TIMEOUT=0; `sccache --stop-server` restarts it)"
        ));
    }
    let errors = after.errors - before.errors;
    if errors > 0 {
        return Err(format!(
            "{errors} cache errors this run, so those compilations ran uncached; fix: check `sccache --show-stats` and the disk under ~/.cache/sccache"
        ));
    }
    Ok(rate(before, after))
}

fn rate(before: Stats, after: Stats) -> String {
    let hits = after.hits - before.hits;
    let misses = after.misses - before.misses;
    let total = hits + misses;
    if total == 0 {
        return "sccache: no compilations this run (everything was fresh)".to_owned();
    }
    let percent = hits * 100 / total;
    format!(
        "sccache: {percent}% hits ({hits} of {total} compilations; the server is shared, so other worktrees' builds meanwhile count)"
    )
}

#[cfg(test)]
#[path = "sccache_tests.rs"]
mod tests;
