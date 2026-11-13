//! Live heap bytes per element after a page's mount and settle
//! (finish-line property 8), from a counting global allocator, which is
//! why it is a binary of its own: `vapors-perf`'s timed runs keep the
//! system allocator. `cargo run --release -p vapors-perf --bin
//! vapors-perf-memory -- --rows <rows>` measures one size and prints its
//! `element-memory` line.

use std::alloc::System;

use stats_alloc::{INSTRUMENTED_SYSTEM, StatsAlloc};
use vapors_gpu_harness::gpu;
use vapors_perf::element_memory_run::run_child;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

/// Bytes allocated less bytes freed since the process started.
fn live() -> i64 {
    let stats = GLOBAL.stats();
    let live = i128::try_from(stats.bytes_allocated).unwrap_or(i128::MAX)
        - i128::try_from(stats.bytes_deallocated).unwrap_or(i128::MAX);
    i64::try_from(live).expect("the live heap fits i64")
}

fn main() {
    if let Err(error) = run() {
        eprintln!("FAIL perf memory: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let rows = rows(std::env::args().skip(1))?;
    println!("{}", run_child(&gpu(), rows, live)?.line());
    Ok(())
}

/// The row count from `--rows <rows>`, alone.
fn rows(mut args: impl Iterator<Item = String>) -> Result<usize, String> {
    const USAGE: &str = "usage: vapors-perf-memory --rows <rows>";
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
