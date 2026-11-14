//! The harness's command line: `[runs] [warmup]`, 200 and 20 by default,
//! with `--only <substring>`, `--instructions`, `--off-path` and
//! `--scaling <change>`, or one of the modes that replace the full run.

pub use crate::args_off_path::WorkloadSet;
use crate::args_off_path::refuse_off_path_with_modes;
use crate::rows::RowChange;
use crate::run::Settings;
use crate::scene_kind::SceneKind;

/// The most timed frames: the GPU clock writes two timestamps a frame
/// into one query set, which holds at most `QUERY_SET_MAX_QUERIES`.
pub const MAX_RUNS: usize = (wgpu::QUERY_SET_MAX_QUERIES / 2) as usize;

pub(crate) const USAGE: &str = "cargo run --release -p vapors-perf -- \
                     [runs] [warmup] [--only <substring> (once)] [--instructions] [--off-path] \
                     [--scaling all|Idle|Text|FlexText|Class|HoveredClass|Insert|FlexInsert|Resize|Scroll|ScrollSize] [--mount] [--mount-memory <rows>] \
                     [--scene-run [racer|one-scene]]";

/// What the command line asks for.
#[derive(Debug)]
pub struct Options {
    /// Frames to run and keep.
    pub settings: Settings,
    /// Run only workloads whose name contains this.
    pub only: Option<String>,
    /// Report instructions retired per phase instead of times.
    pub instructions: bool,
    /// Which workloads the full run runs.
    pub set: WorkloadSet,
    /// Run these changes at each size of `scaling::SIZES` and judge their
    /// growth: one named change, or [`RowChange::ALL`] for `--scaling all`.
    pub scaling: Option<Vec<RowChange>>,
    /// Run the Mount workload at each size of `scaling::SIZES`.
    pub mount: bool,
    /// Run the Mount workload at one size, in this process, and print its
    /// resident set on one line (`mount_memory`'s child mode).
    pub mount_memory: Option<usize>,
    /// Run a scene for 60 s of frames, every one judged (`scene_run`):
    /// the racer, or the scene named after the flag; no counts, no other
    /// flag.
    pub scene_run: Option<SceneKind>,
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
        set,
        scaling,
        mount,
        mount_memory,
        scene_run,
    } = flags;
    refuse_scaling_with_others(scaling.is_some(), only.is_some(), instructions)?;
    refuse_off_path_with_modes(
        set,
        [
            ("--scaling", scaling.is_some()),
            ("--mount", mount),
            ("--mount-memory", mount_memory.is_some()),
            ("--scene-run", scene_run.is_some()),
        ],
    )?;
    refuse_mount_with_others([
        ("--only", only.is_some()),
        ("--instructions", instructions),
        ("--scaling", scaling.is_some()),
        ("--mount", mount),
        ("--mount-memory", mount_memory.is_some()),
        ("--scene-run", scene_run.is_some()),
    ])?;
    refuse_counts_with_scene_run(scene_run.is_some(), &counts)?;
    Ok(Options {
        settings: settings(counts.into_iter())?,
        only,
        instructions,
        set,
        scaling,
        mount,
        mount_memory,
        scene_run,
    })
}

#[derive(Default)]
struct Flags {
    counts: Vec<String>,
    only: Option<String>,
    instructions: bool,
    set: WorkloadSet,
    scaling: Option<Vec<RowChange>>,
    mount: bool,
    mount_memory: Option<usize>,
    scene_run: Option<SceneKind>,
}

impl Flags {
    /// Records `arg`, taking its value from `rest` when it has one.
    fn take(&mut self, arg: String, rest: &mut impl Iterator<Item = String>) -> Result<(), String> {
        match arg.as_str() {
            "--only" if self.only.is_some() => return Err(format!("--only given twice: {USAGE}")),
            "--only" => self.only = Some(only_value(rest.next())?),
            "--instructions" => self.instructions = true,
            "--off-path" => self.set = WorkloadSet::OffPath,
            "--mount" => self.mount = true,
            "--scene-run" => return self.scene_run(rest),
            "--mount-memory" => self.mount_memory = Some(mount_rows(rest.next())?),
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

impl Flags {
    /// `--scene-run`, and the scene named after it: the racer when the
    /// next argument is a flag or a count (read as given then), or there
    /// is none.
    fn scene_run(&mut self, rest: &mut impl Iterator<Item = String>) -> Result<(), String> {
        if self.scene_run.is_some() {
            return Err(format!("--scene-run given twice: {USAGE}"));
        }
        self.scene_run = Some(SceneKind::Racer);
        let Some(next) = rest.next() else {
            return Ok(());
        };
        if let Some(kind) = SceneKind::named(&next) {
            self.scene_run = Some(kind);
            return Ok(());
        }
        if next.starts_with("--") || next.parse::<usize>().is_ok() {
            return self.take(next, rest);
        }
        Err(format!(
            "--scene-run {next}: no scene is called `{next}`; the scenes are racer and one-scene: {USAGE}"
        ))
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

/// `--mount`, `--mount-memory` and `--scene-run` each replace the full
/// run with fixed sizes or one given size: nothing to select or add.
/// `modes` is every flag that selects or replaces, with whether it was
/// given, in the order they may follow: each from the fourth is refused
/// with the flags before it.
fn refuse_mount_with_others(modes: [(&str, bool); 6]) -> Result<(), String> {
    for (at, (mode, _)) in modes.iter().enumerate().skip(3) {
        let given = modes[at].1;
        let other = modes[..at]
            .iter()
            .find_map(|&(flag, on)| on.then_some(flag));
        if let (true, Some(other)) = (given, other) {
            return Err(format!("{mode} and {other} cannot be combined: {USAGE}"));
        }
    }
    Ok(())
}

/// The 60 s run has a fixed length and judges every frame from the
/// first, so it takes neither runs nor warmup.
fn refuse_counts_with_scene_run(scene_run: bool, counts: &[String]) -> Result<(), String> {
    match counts.first() {
        Some(count) if scene_run => Err(format!(
            "--scene-run takes no counts (it runs 3750 frames, no warmup), but {count} was given: {USAGE}"
        )),
        _ => Ok(()),
    }
}

fn mount_rows(value: Option<String>) -> Result<usize, String> {
    value
        .and_then(|rows| rows.parse().ok())
        .ok_or_else(|| format!("--mount-memory needs a row count: {USAGE}"))
}

fn scaling_change(value: Option<String>) -> Result<Vec<RowChange>, String> {
    let name = value.ok_or_else(|| format!("--scaling needs a change: {USAGE}"))?;
    match RowChange::named(&name) {
        _ if name == "all" => Ok(RowChange::ALL.to_vec()),
        Some(change) => Ok(vec![change]),
        None => Err(format!("--scaling {name} is not a change: {USAGE}")),
    }
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

#[cfg(test)]
#[path = "args_scene_run_tests.rs"]
mod scene_run_tests;

#[cfg(test)]
#[path = "args_off_path_tests.rs"]
mod off_path_tests;
