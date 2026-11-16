//! Live heap bytes per element after a page's mount and settle
//! (finish-line property 8), from a counting global allocator, which is
//! why it is a binary of its own: `vapors-perf`'s timed runs keep the
//! system allocator. `cargo run --release -p vapors-perf --bin
//! vapors-perf-memory` measures a 0-row page and 1,000, 3,000 and 10,000
//! rows, each in a child (`--rows <rows>`, which prints one
//! `element-memory` line), prints their table, and fails when the bytes
//! per element grow with the page (`element_memory_flat.rs`).
//! `--add-row --rows <rows>` adds one row to a settled page and prints
//! the bytes it left and the time of its frames.

use std::alloc::System;

use stats_alloc::{INSTRUMENTED_SYSTEM, StatsAlloc};
use vapors_gpu_harness::gpu;
use vapors_perf::element_memory_add::add_row;
use vapors_perf::element_memory_run::{live_bytes, run_child};
use vapors_perf::element_memory_sizes;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn live() -> i64 {
    live_bytes(GLOBAL)
}

fn main() {
    if let Err(error) = run() {
        eprintln!("FAIL perf memory: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        let exe = std::env::current_exe().map_err(|error| format!("this binary: {error}"))?;
        return element_memory_sizes::run(&exe);
    }
    if args.first().map(String::as_str) == Some("--add-row") {
        let rows = rows(args.into_iter().skip(1))?;
        println!("{}", add_row(&gpu(), rows, live)?.line());
        return Ok(());
    }
    let rows = rows(args.into_iter())?;
    println!("{}", run_child(&gpu(), rows, live)?.line());
    Ok(())
}

/// The row count from `--rows <rows>`, alone.
fn rows(mut args: impl Iterator<Item = String>) -> Result<usize, String> {
    const USAGE: &str = "usage: vapors-perf-memory [[--add-row] --rows <rows>]";
    let (Some(flag), Some(value), None) = (args.next(), args.next(), args.next()) else {
        return Err(USAGE.into());
    };
    if flag != "--rows" {
        return Err(format!("{flag}: unknown; {USAGE}"));
    }
    value
        .parse()
        .map_err(|_| format!("--rows {value}: not a row count; {USAGE}"))
}
