//! The property 8 run: a 0-row page and each of `element_memory_flat`'s
//! sizes, each measured by a child process of this binary (`--rows`),
//! their table, and the flat check's verdict.

use std::process::Command;

use crate::element_memory::{ElementMemory, parse_line};
use crate::element_memory_flat::{SIZES, table, violations};

/// How to run it again.
pub const REPRODUCE: &str = "cargo run --release -p vapors-perf --bin vapors-perf-memory";

/// Measures every size in a child of `exe`, prints the table, and
/// judges it.
///
/// # Errors
/// A child that failed or printed no element-memory line, naming it, or
/// the slopes that grew, with the command to run again.
pub fn run(exe: &std::path::Path) -> Result<(), String> {
    let mut pages = vec![child(exe, 0)?];
    for rows in SIZES {
        pages.push(child(exe, rows)?);
    }
    println!("{}", table(&pages));
    let found = violations(&pages[1..])?;
    if found.is_empty() {
        println!("per-element bytes flat across {SIZES:?} rows");
        return Ok(());
    }
    Err(format!(
        "per-element bytes grow with the page:\n{}\nreproduce: {REPRODUCE}",
        found.join("\n")
    ))
}

/// One page of `rows` rows, measured by a child of `exe`.
fn child(exe: &std::path::Path, rows: usize) -> Result<ElementMemory, String> {
    let output = Command::new(exe)
        .args(["--rows", &rows.to_string()])
        .output()
        .map_err(|error| format!("{}: {error}", exe.display()))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "the {rows}-row child failed ({}): {stderr}{stdout}",
            output.status
        ));
    }
    let line = stdout
        .lines()
        .find(|line| line.starts_with("element-memory"));
    parse_line(line.ok_or_else(|| format!("the {rows}-row child printed no line: {stdout}"))?)
}
