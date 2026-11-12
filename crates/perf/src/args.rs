//! The harness's command line: `[runs] [warmup]`, 200 and 20 by default,
//! with `--only <substring>`, `--instructions` and `--scaling <change>`.

use crate::rows::RowChange;
use crate::run::Settings;

/// The most timed frames: the GPU clock writes two timestamps a frame
/// into one query set, which holds at most `QUERY_SET_MAX_QUERIES`.
pub const MAX_RUNS: usize = (wgpu::QUERY_SET_MAX_QUERIES / 2) as usize;

const USAGE: &str = "cargo run --release -p vapors-perf -- \
                     [runs] [warmup] [--only <substring> (once)] [--instructions] \
                     [--scaling Idle|Text|Class|HoveredClass|Insert]";

/// What the command line asks for.
#[derive(Debug)]
pub struct Options {
    /// Frames to run and keep.
    pub settings: Settings,
    /// Run only workloads whose name contains this.
    pub only: Option<String>,
    /// Report instructions retired per phase instead of times.
    pub instructions: bool,
    /// Run this change at each size of `scaling::SIZES`, count its
    /// instructions and judge their growth.
    pub scaling: Option<RowChange>,
}

/// The options `args` name, without the program's name.
///
/// # Errors
/// An unknown flag, `--only` without a value or given twice, an argument that is not a
/// count, runs outside 1 to [`MAX_RUNS`], or a third count; each names
/// the usage.
pub fn options(mut args: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut flags = Flags::default();
    while let Some(arg) = args.next() {
        flags.take(arg, &mut args)?;
    }
    let Flags {
        counts,
        only,
        instructions,
        scaling,
    } = flags;
    refuse_scaling_with_others(scaling.is_some(), only.is_some(), instructions)?;
    Ok(Options {
        settings: settings(counts.into_iter())?,
        only,
        instructions,
        scaling,
    })
}

#[derive(Default)]
struct Flags {
    counts: Vec<String>,
    only: Option<String>,
    instructions: bool,
    scaling: Option<RowChange>,
}

impl Flags {
    /// Records `arg`, taking its value from `rest` when it has one.
    fn take(&mut self, arg: String, rest: &mut impl Iterator<Item = String>) -> Result<(), String> {
        match arg.as_str() {
            "--only" if self.only.is_some() => return Err(format!("--only given twice: {USAGE}")),
            "--only" => self.only = Some(only_value(rest.next())?),
            "--instructions" => self.instructions = true,
            "--scaling" if self.scaling.is_some() => {
                return Err(format!("--scaling given twice: {USAGE}"));
            }
            "--scaling" => self.scaling = Some(scaling_change(rest.next())?),
            flag if flag.starts_with("--") => return Err(format!("unknown flag {flag}: {USAGE}")),
            _ => self.counts.push(arg),
        }
        Ok(())
    }
}

/// `--scaling` always counts instructions, over a fixed set of sizes and
/// workloads, so `--only` and `--instructions` have nothing to select or add.
fn refuse_scaling_with_others(scaling: bool, only: bool, instructions: bool) -> Result<(), String> {
    if scaling && only {
        return Err(format!("--scaling and --only cannot be combined: {USAGE}"));
    }
    if scaling && instructions {
        return Err(format!(
            "--scaling and --instructions cannot be combined (--scaling always counts instructions): {USAGE}"
        ));
    }
    Ok(())
}

fn scaling_change(value: Option<String>) -> Result<RowChange, String> {
    let name = value.ok_or_else(|| format!("--scaling needs a change: {USAGE}"))?;
    RowChange::named(&name).ok_or_else(|| format!("--scaling {name} is not a change: {USAGE}"))
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
