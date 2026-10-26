//! Room on the disk for a gate. On 2026-10-26 the disk filled to 0 bytes
//! at about 07:12Z and every gate failed with ENOSPC, so the queue checks
//! free space (`df`) before it gates. Below `MIN_FREE` it does not gate,
//! and it logs the largest target directories (`du`) under the roots in
//! `[queue] targets`, so whoever reads the log knows what to empty.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::queue_log::Log;
use crate::queue_ops::Ops;
use crate::queue_state::State;

/// A gigabyte, as `df` and `du` count with `-B1`.
pub const GB: u64 = 1_000_000_000;

/// The free space below which the queue does not gate: the lead's figure
/// after the 2026-10-26 ENOSPC. That day the largest touche worktree's
/// target held 62 GB, so 60 GB is about one gate's build from scratch.
pub const MIN_FREE: u64 = 60 * GB;

/// How many of the largest target directories the log names.
const NAMED: usize = 5;

/// The free bytes in `df -B1 --output=avail`'s output.
///
/// # Errors
///
/// The output is not a header line and one number, quoted.
pub fn parse_df(text: &str) -> Result<u64, String> {
    let refused = || format!("df's output is not a header and one number: {text:?}");
    let mut lines = text.lines();
    match (lines.next(), lines.next(), lines.next()) {
        (Some(_), Some(free), None) => free.trim().parse().map_err(|_| refused()),
        _ => Err(refused()),
    }
}

/// `du -s -B1`'s lines as sizes and paths, largest first.
///
/// # Errors
///
/// A line that is not a size, a tab and a path, quoted.
pub fn parse_du(text: &str) -> Result<Vec<(u64, PathBuf)>, String> {
    let mut sizes = text.lines().map(du_line).collect::<Result<Vec<_>, _>>()?;
    sizes.sort_by(|a, b| b.cmp(a));
    Ok(sizes)
}

fn du_line(line: &str) -> Result<(u64, PathBuf), String> {
    line.split_once('\t')
        .and_then(|(size, path)| Some((size.parse().ok()?, PathBuf::from(path))))
        .ok_or_else(|| format!("du's line is not a size, a tab and a path: {line:?}"))
}

/// The log's line for a disk with `free` bytes and these target sizes.
#[must_use]
pub fn describe(free: u64, largest: &[(u64, PathBuf)]) -> String {
    let named: Vec<String> = largest
        .iter()
        .take(NAMED)
        .map(|(size, path)| format!("{} GB {}", size / GB, path.display()))
        .collect();
    let named = if named.is_empty() {
        "none found".to_owned()
    } else {
        named.join(", ")
    };
    format!(
        "the disk has {} GB free; the largest targets: {named}; fix: empty the ones whose branch merged (find <dir> -mindepth 1 -maxdepth 1 -exec rm -rf {{}} +)",
        free / GB
    )
}

/// Whether the disk has `MIN_FREE` for a gate. When not, it says so once
/// for PR `number`, naming the largest targets, and the PR waits. A df or
/// du that fails is said too, and the PR waits; the pass goes on. df
/// reads the run worktree's filesystem, which on the box is the targets
/// roots' too (both /dev/nvme0n1p2, 2026-10-26): a declared limit.
///
/// # Errors
///
/// The log failing.
pub fn room_to_gate(
    ops: &impl Ops,
    log: &Log,
    state: &mut State,
    number: u64,
) -> Result<bool, String> {
    let free = match ops.free_bytes() {
        Ok(free) => free,
        Err(why) => {
            let reason = format!("#{number} is not gated: df failed: {why}");
            if state.changed(number, &reason) {
                log.line(&reason)?;
            }
            return Ok(false);
        }
    };
    if free < MIN_FREE {
        say_short(ops, log, state, (number, free))?;
    }
    Ok(free >= MIN_FREE)
}

/// Say once per PR that the disk is short, with the largest targets; the
/// reason is recorded only once both lines are written, so a du that
/// fails is run again next pass.
fn say_short(
    ops: &impl Ops,
    log: &Log,
    state: &mut State,
    (number, free): (u64, u64),
) -> Result<(), String> {
    let reason = format!(
        "#{number} is not gated: the disk has under {} GB free",
        MIN_FREE / GB
    );
    if state.said_already(number, &reason) {
        return Ok(());
    }
    log.line(&reason)?;
    match ops.largest_targets() {
        Ok(largest) => {
            log.line(&describe(free, &largest))?;
            state.changed(number, &reason);
            Ok(())
        }
        Err(why) => log.line(&format!(
            "the disk has {} GB free; du failed: {why}; it is tried again next pass",
            free / GB
        )),
    }
}

/// df's deadline. Measured 2026-10-26: 4 to 13 ms over three, at a load
/// of 39; 30 s is far over that, for a disk that has stopped answering.
const DF_DEADLINE: Duration = Duration::from_secs(30);

/// du's deadline. Measured 2026-10-26: `du -s` over the 160 target
/// directories (about 600 GB) took 10.6 s twice; 120 s is 11 times that.
const DU_DEADLINE: Duration = Duration::from_secs(120);

/// The free bytes on the disk holding `path`.
///
/// # Errors
///
/// df failing, timing out, or answering in another shape.
pub fn free_bytes(path: &Path) -> Result<u64, String> {
    let mut command = Command::new("df");
    command.args(["-B1", "--output=avail"]).arg(path);
    parse_df(&crate::queue_remote::within(command, "df", DF_DEADLINE)?)
}

/// The directories in `roots` with their sizes, largest first.
///
/// # Errors
///
/// A root that cannot be read, or du failing or timing out.
pub fn largest(roots: &[PathBuf]) -> Result<Vec<(u64, PathBuf)>, String> {
    let dirs = children(roots)?;
    if dirs.is_empty() {
        return Ok(Vec::new());
    }
    let mut command = Command::new("du");
    command.args(["-s", "-B1"]).args(&dirs);
    parse_du(&crate::queue_remote::within(command, "du", DU_DEADLINE)?)
}

fn children(roots: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut dirs = Vec::new();
    for root in roots {
        let named = |e: std::io::Error| format!("{}: {e}", root.display());
        for entry in std::fs::read_dir(root).map_err(named)? {
            dirs.push(entry.map_err(named)?.path());
        }
    }
    Ok(dirs)
}

#[cfg(test)]
#[path = "queue_disk_tests.rs"]
mod tests;
