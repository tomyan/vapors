//! Prints the timing baseline (docs/perf): every workload, in a release
//! build. `cargo run --release -p vapors-perf -- [runs] [warmup]
//! [--only <substring>] [--instructions|--counters] [--off-path] [--scaling <change>] [--mount]
//! [--mount-memory <rows>] [--scene-run]`, 200 and 20 by default
//! (`args.rs`).

use vapors_gpu_harness::gpu_with;
use vapors_perf::args::{Measure, Options, WorkloadSet, options};
use vapors_perf::frame_kinds::sections;
use vapors_perf::gpu_time::FEATURES;
use vapors_perf::hardware_counters::HardwareCounters;
use vapors_perf::instructions_check::checked_counter;
use vapors_perf::mount_memory;
use vapors_perf::mount_run;
use vapors_perf::off_path::{OFF_PATH_SETUP, off_path_workloads};
use vapors_perf::run::Settings;
use vapors_perf::sampler::Sampler;
use vapors_perf::scaling_run;
use vapors_perf::scene_run;
use vapors_perf::select::matching;
use vapors_perf::spans::PhaseClock;
use vapors_perf::workloads::{settings_for, workloads};

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
        measure,
        set,
        scaling,
        mount,
        mount_memory,
        scene_run,
    } = options(std::env::args().skip(1))?;
    if scene_run {
        return scene_run::run(|| {
            let gpu = gpu_with(FEATURES);
            print_adapter(&gpu);
            gpu
        });
    }
    if let Some(rows) = mount_memory {
        let gpu = gpu_with(FEATURES);
        println!("{}", mount_memory::run_child(&gpu, rows)?.line());
        return Ok(());
    }
    if mount {
        let gpu = gpu_with(FEATURES);
        print_setup(&gpu, settings);
        return mount_run::run(&gpu, settings);
    }
    if let Some(change) = scaling {
        let gpu = gpu_with(FEATURES);
        print_setup(&gpu, settings);
        return scaling_run::run(&gpu, change, settings);
    }
    let all = match set {
        WorkloadSet::Main => workloads(),
        WorkloadSet::OffPath => off_path_workloads(),
    };
    let chosen = matching(all, only.as_deref())?;
    let clock = match measure {
        Measure::Times => PhaseClock::default(),
        Measure::Instructions => PhaseClock::with_instructions(checked_counter()?),
        Measure::Counters => PhaseClock::with_counters(HardwareCounters::open()?),
    };
    let gpu = gpu_with(FEATURES);
    print_setup(&gpu, settings);
    let mut timer = Sampler::with_clock(&gpu, clock);
    if set == WorkloadSet::OffPath {
        println!("{OFF_PATH_SETUP}\n");
    }
    for (name, (rows, make)) in chosen {
        let mut workload = make();
        let at = settings_for(rows, settings);
        let samples = timer
            .sample(workload.as_mut(), at)
            .map_err(|e| format!("{name}: {e}"))?;
        let kinds = workload.frame_kinds();
        for report in sections(&name, &samples, kinds, measure)? {
            println!("{report}");
        }
    }
    println!("Load average at the end: {}", load());
    Ok(())
}

fn print_setup(gpu: &vapors_gpu_harness::Gpu, settings: Settings) {
    print_adapter(gpu);
    println!(
        "Profile: release; scale 1; warmup {}; runs {}",
        settings.warmup, settings.runs
    );
    println!("Load average at the start: {}\n", load());
}

fn print_adapter(gpu: &vapors_gpu_harness::Gpu) {
    let info = gpu.adapter.get_info();
    println!(
        "Adapter: {} ({:?}, {} {})",
        info.name, info.backend, info.driver, info.driver_info
    );
    println!(
        "Timestamp queries: {}",
        gpu.device.features().contains(FEATURES)
    );
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
