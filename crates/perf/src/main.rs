//! Prints the timing baseline (docs/perf): every workload, in a release
//! build. `cargo run --release -p vapors-perf -- [runs] [warmup]`, 200
//! and 20 by default.

use vapors_gpu_harness::gpu_with;
use vapors_perf::gpu_time::FEATURES;
use vapors_perf::racer::RacerFrames;
use vapors_perf::report::section;
use vapors_perf::rows::{RowChange, Rows};
use vapors_perf::run::{Settings, Workload};
use vapors_perf::sampler::Sampler;

fn main() {
    if let Err(error) = run() {
        eprintln!("FAIL perf: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    if cfg!(debug_assertions) {
        return Err("a debug build; the baseline is timed in release: \
                    cargo run --release -p vapors-perf"
            .into());
    }
    let settings = settings(std::env::args().skip(1))?;
    let gpu = gpu_with(FEATURES);
    print_setup(&gpu, settings);
    let mut timer = Sampler::new(&gpu);
    for (name, mut workload) in workloads() {
        let samples = timer
            .sample(workload.as_mut(), settings)
            .map_err(|e| format!("{name}: {e}"))?;
        println!("{}", section(&name, &samples));
    }
    println!("Load average at the end: {}", load());
    Ok(())
}

fn settings(mut args: impl Iterator<Item = String>) -> Result<Settings, String> {
    let mut number = |default: usize| {
        args.next().map_or(Ok(default), |arg| {
            arg.parse().map_err(|_| format!("{arg} is not a count"))
        })
    };
    Ok(Settings {
        runs: number(200)?,
        warmup: number(20)?,
    })
}

fn workloads() -> Vec<(String, Box<dyn Workload>)> {
    let mut all: Vec<(String, Box<dyn Workload>)> = Vec::new();
    for count in [100, 1000, 3000] {
        for change in [RowChange::Idle, RowChange::Text, RowChange::Class] {
            all.push((
                format!("{count} rows, {change:?}"),
                Box::new(Rows::new(count, change)),
            ));
        }
    }
    all.push(("racer, paused".into(), Box::new(RacerFrames::new(false))));
    all.push(("racer, racing".into(), Box::new(RacerFrames::new(true))));
    all
}

fn print_setup(gpu: &vapors_gpu_harness::Gpu, settings: Settings) {
    let info = gpu.adapter.get_info();
    println!(
        "Adapter: {} ({:?}, {} {})",
        info.name, info.backend, info.driver, info.driver_info
    );
    println!(
        "Timestamp queries: {}",
        gpu.device.features().contains(FEATURES)
    );
    println!(
        "Profile: release; scale 1; warmup {}; runs {}",
        settings.warmup, settings.runs
    );
    println!("Load average at the start: {}\n", load());
}

/// The first three fields of /proc/loadavg, or why they could not be read.
fn load() -> String {
    std::fs::read_to_string("/proc/loadavg").map_or_else(
        |error| format!("unread ({error})"),
        |line| {
            line.split_whitespace()
                .take(3)
                .collect::<Vec<_>>()
                .join(" ")
        },
    )
}
