//! The harness's command line: `[runs] [warmup]`, 200 and 20 by default,
//! with `--only <substring>` and `--instructions`.

use crate::run::Settings;

/// The most timed frames: the GPU clock writes two timestamps a frame
/// into one query set, which holds at most `QUERY_SET_MAX_QUERIES`.
pub const MAX_RUNS: usize = (wgpu::QUERY_SET_MAX_QUERIES / 2) as usize;

const USAGE: &str = "cargo run --release -p vapors-perf -- \
                     [runs] [warmup] [--only <substring>] [--instructions]";

/// What the command line asks for.
#[derive(Debug)]
pub struct Options {
    /// Frames to run and keep.
    pub settings: Settings,
    /// Run only workloads whose name contains this.
    pub only: Option<String>,
    /// Report instructions retired per phase instead of times.
    pub instructions: bool,
}

/// The options `args` name, without the program's name.
///
/// # Errors
/// An unknown flag, `--only` without a value, an argument that is not a
/// count, runs outside 1 to [`MAX_RUNS`], or a third count; each names
/// the usage.
pub fn options(mut args: impl Iterator<Item = String>) -> Result<Options, String> {
    let (mut counts, mut only, mut instructions) = (Vec::new(), None, false);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--only" => only = Some(only_value(args.next())?),
            "--instructions" => instructions = true,
            flag if flag.starts_with("--") => return Err(format!("unknown flag {flag}: {USAGE}")),
            _ => counts.push(arg),
        }
    }
    let settings = settings(counts.into_iter())?;
    Ok(Options {
        settings,
        only,
        instructions,
    })
}

fn only_value(value: Option<String>) -> Result<String, String> {
    value.ok_or_else(|| format!("--only needs a value: {USAGE}"))
}

fn settings(mut args: impl Iterator<Item = String>) -> Result<Settings, String> {
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
