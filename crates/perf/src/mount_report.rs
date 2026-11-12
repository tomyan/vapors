//! A Mount run as Markdown: instructions per phase at each size and per
//! extra row, and the phase that dominates.

use std::collections::BTreeMap;
use std::fmt::Write;

use crate::scaling::{AtSize, Scaling};

/// One size's measurement: median instructions per phase (the mount
/// under "mount", the first frame's phases by their names).
#[derive(Debug)]
pub struct Measured {
    /// The page's row count.
    pub rows: usize,
    /// Median instructions by phase name.
    pub instructions: BTreeMap<String, u64>,
}

/// The phases of a first frame: every one, as `accessibility` builds
/// its tree there and the steady-frame table (`report::PHASES`) leaves
/// the app's other phases out.
pub const MOUNT_PHASES: [&str; 13] = crate::report::EVERY_PHASE;

/// Phases that do not contain each other, whose sum is the work from
/// mount to the end of the first draw. `update`'s own work is `update`
/// minus the three spans inside it ([`update_rest`]).
const LEAVES: [&str; 10] = [
    "mount",
    "input",
    "tasks",
    "tick",
    "style",
    "layout",
    "paint",
    "accessibility",
    "draw",
    "record",
];

/// Instructions per phase at each size, with the growth per extra row
/// between neighbouring sizes.
///
/// # Errors
/// Sizes that are not ascending, or a phase missing at one size.
pub fn instructions_table(sizes: &[Measured]) -> Result<String, String> {
    let at_sizes: Vec<AtSize> = sizes
        .iter()
        .map(|m| AtSize {
            rows: m.rows,
            phases: m.instructions.clone(),
        })
        .collect();
    let scaling = Scaling::of(&at_sizes)?;
    let mut out = String::from("| phase |");
    for rows in &scaling.rows {
        let _ = write!(out, " {rows} rows |");
    }
    for pair in scaling.rows.windows(2) {
        let _ = write!(out, " per extra row, {} to {} |", pair[0], pair[1]);
    }
    let _ = writeln!(out, "\n{}|", "|---".repeat(scaling.rows.len() * 2));
    for series in &scaling.phases {
        let _ = write!(out, "| {} |", series.phase);
        for count in &series.counts {
            let _ = write!(out, " {count} |");
        }
        for at in 1..scaling.rows.len() {
            let _ = write!(out, " {:.1} |", series.per_row(&scaling.rows, at - 1, at));
        }
        out.push('\n');
    }
    Ok(out)
}

/// `update` minus the spans inside it: its own work. `None` when the
/// measurement has no `update`.
fn update_rest(instructions: &BTreeMap<String, u64>) -> Option<u64> {
    let inside: u64 = ["style", "layout", "paint"]
        .iter()
        .filter_map(|phase| instructions.get(*phase))
        .sum();
    Some(instructions.get("update")?.saturating_sub(inside))
}

/// The leaf phase with the most instructions at the largest size, as a
/// sentence with its share of the leaves' total.
///
/// # Errors
/// No sizes, or no leaf phase was measured.
pub fn dominant(sizes: &[Measured]) -> Result<String, String> {
    let last = sizes.last().ok_or("mount: no sizes were measured")?;
    let mut leaves: Vec<(&str, u64)> = LEAVES
        .iter()
        .filter_map(|phase| Some((*phase, *last.instructions.get(*phase)?)))
        .collect();
    leaves.extend(update_rest(&last.instructions).map(|rest| ("update (rest)", rest)));
    let total: u64 = leaves.iter().map(|(_, count)| count).sum();
    let (phase, count) = leaves
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .ok_or("mount: none of the leaf phases was measured")?;
    Ok(format!(
        "{phase} dominates at {} rows: {count} of {total} instructions",
        last.rows
    ))
}

#[cfg(test)]
#[path = "mount_report_tests.rs"]
mod tests;
