//! The harness's command line: `[runs] [warmup]`, 200 and 20 by default.

use crate::run::Settings;

/// The most timed frames: the GPU clock writes two timestamps a frame
/// into one query set, which holds at most `QUERY_SET_MAX_QUERIES`.
pub const MAX_RUNS: usize = (wgpu::QUERY_SET_MAX_QUERIES / 2) as usize;

const USAGE: &str = "cargo run --release -p vapors-perf -- [runs] [warmup]";

/// The settings `args` name, without the program's name.
///
/// # Errors
/// An argument that is not a count, runs outside 1 to [`MAX_RUNS`], or
/// a third argument; each names the usage.
pub fn settings(mut args: impl Iterator<Item = String>) -> Result<Settings, String> {
    let runs = count(args.next(), 200)?;
    let warmup = count(args.next(), 20)?;
    if let Some(extra) = args.next() {
        return Err(format!("unexpected argument {extra}: {USAGE}"));
    }
    if !(1..=MAX_RUNS).contains(&runs) {
        return Err(format!("runs is {runs}; give 1 to {MAX_RUNS}: {USAGE}"));
    }
    Ok(Settings { warmup, runs })
}

fn count(arg: Option<String>, default: usize) -> Result<usize, String> {
    arg.map_or(Ok(default), |arg| {
        arg.parse()
            .map_err(|_| format!("{arg} is not a count: {USAGE}"))
    })
}

#[cfg(test)]
#[path = "args_tests.rs"]
mod tests;
