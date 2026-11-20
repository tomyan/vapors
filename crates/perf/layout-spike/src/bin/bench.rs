//! Prints the layout timings as a markdown table.
//! Reproduce: cargo run --offline --profile timing --features all --bin bench
//! (from crates/perf/layout-spike); docs/plans/layout-comparison.md.
//! Exits 1 if an option's boxes differ from the others' or the layout-phase
//! clock saw no layout span; 2 in a debug build.

use std::process::exit;
use std::time::Duration;

use vapors_layout_spike::measure::{Clock, Miss, first, relayout};
use vapors_layout_spike::spec::{DEEP_DEPTH, TABLE_SIZES};
use vapors_layout_spike::taffy_engine::Taffy;
use vapors_layout_spike::{Engine, Workload, agree, clock, fixed, generated, lanes, stats};

fn micros(time: Duration) -> String {
    format!("{:.1}", time.as_secs_f64() * 1e6)
}

#[allow(clippy::unnecessary_wraps)] // the `Clock` signature
fn plain(work: &mut dyn FnMut()) -> Result<Duration, String> {
    Ok(stats::timed(work))
}

fn layout_phase(work: &mut dyn FnMut()) -> Result<Duration, String> {
    clock::layout_phase(work).map_err(|e| e.to_string())
}

/// One markdown row: the option on the workload, `n/a` where it refuses;
/// a clock that saw nothing stops the run.
fn row<E: Engine>(workload: Workload, label: &str, clock: Clock) {
    let cell = |r: Result<Duration, Miss>| match r {
        Ok(time) => micros(time),
        Err(Miss::Unsupported(_)) => "n/a".to_string(),
        Err(Miss::Clock(why)) => {
            eprintln!("{label} on {workload:?}: {why}");
            exit(1);
        }
    };
    println!(
        "| {workload:?} | {label} | {} | {} |",
        cell(first::<E>(workload, clock)),
        cell(relayout::<E>(workload, clock))
    );
}

fn machine() -> String {
    let info = std::fs::read_to_string("/proc/cpuinfo").expect("read /proc/cpuinfo: run on Linux");
    let model = info
        .lines()
        .find(|l| l.starts_with("model name"))
        .and_then(|l| l.split(": ").nth(1));
    let threads = std::thread::available_parallelism().map_or(0, usize::from);
    format!("{} ({threads} threads)", model.unwrap_or("unknown cpu"))
}

fn main() {
    if cfg!(debug_assertions) {
        eprintln!(
            "a debug build's times mean nothing; run: \
             cargo run --offline --profile timing --features all --bin bench"
        );
        exit(2);
    }
    let workloads: Vec<Workload> = TABLE_SIZES
        .map(Workload::Table)
        .into_iter()
        .chain([Workload::Deep(DEEP_DEPTH)])
        .collect();
    for &workload in &workloads {
        if let Err(why) = agree::check_all(workload) {
            eprintln!("not timing: {why}");
            exit(1);
        }
    }
    println!("machine: {}", machine());
    println!(
        "| workload | option | first layout (us) | relayout after one leaf (us) |\n|---|---|---|---|"
    );
    for workload in workloads {
        row::<Taffy>(workload, "taffy", plain);
        row::<lanes::Lanes>(workload, "lanes, whole update", plain);
        row::<lanes::Lanes>(workload, "lanes, layout phase only", layout_phase);
        row::<fixed::Fixed>(workload, "compile-time boundary", plain);
        row::<generated::Generated>(workload, "generated per component", plain);
    }
}
