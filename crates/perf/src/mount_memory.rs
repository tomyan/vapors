//! The Mount workload's memory, one size to a process. A page's frame
//! allocates far more than its mount keeps, and the allocator hands
//! that memory to the next page, so a second size run in the same
//! process would start with the first one's heap. `--mount` starts a
//! process per size (`--mount-memory <rows>`), which prints one line
//! this module parses.

use std::fmt::Write;
use std::process::Command;

use vapors_gpu_harness::Gpu;

use crate::instructions_check::checked_counter;
use crate::mount::Mount;
use crate::run::Settings;
use crate::sampler::Sampler;

/// Prefix of the line a child prints.
const PREFIX: &str = "mount-memory";

/// The fields of a line after its prefix, in order.
const FIELDS: [&str; 5] = ["rows", "before", "after_mount", "after_frame", "peak"];

/// A page's resident set around its mount and first frame, KiB.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryRow {
    /// The page's row count.
    pub rows: usize,
    /// Before the mount.
    pub before_kib: u64,
    /// After the mount.
    pub after_mount_kib: u64,
    /// After the first frame.
    pub after_frame_kib: u64,
    /// The process's peak so far, after the first frame.
    pub peak_kib: u64,
}

impl MemoryRow {
    /// The line a child prints for this row.
    #[must_use]
    pub fn line(&self) -> String {
        format!(
            "{PREFIX} rows={} before={} after_mount={} after_frame={} peak={}",
            self.rows, self.before_kib, self.after_mount_kib, self.after_frame_kib, self.peak_kib
        )
    }
}

/// The row a line spells.
///
/// # Errors
/// A line that is not [`MemoryRow::line`]'s shape, naming the line.
pub fn parse_line(line: &str) -> Result<MemoryRow, String> {
    let refuse = || format!("not a {PREFIX} line of {FIELDS:?}: {line:?}");
    let rest = line.strip_prefix(PREFIX).ok_or_else(refuse)?;
    let values = rest
        .split_whitespace()
        .zip(FIELDS)
        .map(|(pair, field)| pair.strip_prefix(field)?.strip_prefix('=')?.parse().ok())
        .collect::<Option<Vec<u64>>>()
        .filter(|values| {
            values.len() == FIELDS.len() && rest.split_whitespace().count() == FIELDS.len()
        })
        .ok_or_else(refuse)?;
    Ok(MemoryRow {
        rows: usize::try_from(values[0]).map_err(|_| refuse())?,
        before_kib: values[1],
        after_mount_kib: values[2],
        after_frame_kib: values[3],
        peak_kib: values[4],
    })
}

/// One size in this process: the line `--mount-memory` prints.
///
/// # Errors
/// A mount or frame that failed, or a resident set that cannot be read.
pub fn run_child(gpu: &Gpu, rows: usize) -> Result<MemoryRow, String> {
    let mut workload = Mount::new(rows, checked_counter()?);
    Sampler::new(gpu).sample(&mut workload, Settings { warmup: 0, runs: 1 })?;
    let first = workload.mounted[0];
    let framed = workload.after_frame.borrow()[0];
    Ok(MemoryRow {
        rows,
        before_kib: first.rss_before_kib,
        after_mount_kib: first.rss_after_kib,
        after_frame_kib: framed.resident_kib,
        peak_kib: framed.peak_kib,
    })
}

/// Each of `sizes` in a process of its own.
///
/// # Errors
/// A child that cannot start, exits with a failure (its stderr is
/// included) or prints no memory line.
pub fn in_processes(sizes: &[usize]) -> Result<Vec<MemoryRow>, String> {
    let exe = std::env::current_exe().map_err(|e| format!("finding this program: {e}"))?;
    sizes.iter().map(|&rows| child(&exe, rows)).collect()
}

fn child(exe: &std::path::Path, rows: usize) -> Result<MemoryRow, String> {
    let output = Command::new(exe)
        .args(["--mount-memory", &rows.to_string()])
        .output()
        .map_err(|e| format!("starting {} for {rows} rows: {e}", exe.display()))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "the {rows}-row process failed ({}): {stderr}",
            output.status
        ));
    }
    let line = stdout.lines().rev().find(|line| line.starts_with(PREFIX));
    parse_line(line.ok_or_else(|| format!("the {rows}-row process printed no {PREFIX} line"))?)
}

/// The rows as a table: the resident set at each point, the growth at
/// the mount, and the bytes per row the whole first frame grew it by.
#[must_use]
pub fn memory_table(rows: &[MemoryRow]) -> String {
    let mut out = String::from(
        "| rows | before KiB | after mount KiB | after first frame KiB | peak KiB | mount growth KiB | bytes per row, mount and frame |\n|---|---|---|---|---|---|---|\n",
    );
    for row in rows {
        let mount = row.after_mount_kib.saturating_sub(row.before_kib);
        let frame = row.after_frame_kib.saturating_sub(row.before_kib);
        let per_row = frame * 1024 / row.rows.max(1) as u64;
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {mount} | {per_row} |",
            row.rows, row.before_kib, row.after_mount_kib, row.after_frame_kib, row.peak_kib
        );
    }
    out
}

#[cfg(test)]
#[path = "mount_memory_tests.rs"]
mod tests;
