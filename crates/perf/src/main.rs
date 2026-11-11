//! Prints the timing baseline (docs/perf): every workload, in a release
//! build. `cargo run --release -p vapors-perf -- [runs] [warmup]
//! [--only <substring>] [--instructions]`, 200 and 20 by default
//! (`args.rs`).

use vapors_gpu_harness::gpu_with;
use vapors_perf::args::{Options, options};
use vapors_perf::gpu_time::FEATURES;
use vapors_perf::instructions::InstructionCounter;
use vapors_perf::instructions_check::{phases_counted, self_check};
use vapors_perf::racer::RacerFrames;
use vapors_perf::report::{instructions_section, section};
use vapors_perf::rows::{RowChange, Rows};
use vapors_perf::run::{Samples, Settings, Workload};
use vapors_perf::sampler::Sampler;
use vapors_perf::select::matching;
use vapors_perf::spans::PhaseClock;

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
    let Options {
        settings,
        only,
        instructions,
    } = options(std::env::args().skip(1))?;
    let chosen = matching(workloads(), only.as_deref())?;
    let clock = if instructions {
        PhaseClock::with_instructions(checked_counter()?)
    } else {
        PhaseClock::default()
    };
    let gpu = gpu_with(FEATURES);
    print_setup(&gpu, settings);
    let mut timer = Sampler::with_clock(&gpu, clock);
    for (name, mut workload) in chosen {
        let samples = timer
            .sample(workload.as_mut(), settings)
            .map_err(|e| format!("{name}: {e}"))?;
        let report = if instructions {
            every_phase_counted(&name, &samples)?;
            instructions_section(&name, &samples)?
        } else {
            section(&name, &samples)
        };
        println!("{report}");
    }
    println!("Load average at the end: {}", load());
    Ok(())
}

fn checked_counter() -> Result<InstructionCounter, String> {
    let mut counter = InstructionCounter::open()?;
    self_check(&mut counter)?;
    Ok(counter)
}

fn every_phase_counted(name: &str, samples: &Samples) -> Result<(), String> {
    let counted = samples.instructions.as_ref().map(|kept| &kept.phases);
    let empty = std::collections::BTreeMap::new();
    phases_counted(&samples.phases, counted.unwrap_or(&empty)).map_err(|e| format!("{name}: {e}"))
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
    for count in [1000, 3000] {
        let rows = Rows::new(count, RowChange::HoveredClass);
        all.push((format!("{count} rows, HoveredClass"), Box::new(rows)));
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
