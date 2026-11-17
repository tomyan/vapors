//! Prints the timing baseline (docs/perf): every workload, in a release
//! build. `cargo run --release -p vapors-perf -- [runs] [warmup]
//! [--only <substring>] [--instructions|--counters] [--off-path] [--scaling all|<change>] [--mount]
//! [--mount-memory <rows>] [--first-frame <rows>] [--scene-run [racer|one-scene]]`, 200 and 20 by default
//! (`args.rs`).

use vapors_gpu_harness::gpu_with;
use vapors_perf::args::{Measure, Options, WorkloadSet, options};
use vapors_perf::build_profile;
use vapors_perf::first_frame_run;
use vapors_perf::frame_kinds::sections;
use vapors_perf::gpu_time::FEATURES;
use vapors_perf::hardware_counters::HardwareCounters;
use vapors_perf::instructions_check::checked_counter;
use vapors_perf::mount_memory;
use vapors_perf::mount_run;
use vapors_perf::off_path::{OFF_PATH_SETUP, off_path_workloads};
use vapors_perf::run::Settings;
use vapors_perf::sampler::Sampler;
use vapors_perf::scaling_all;
use vapors_perf::scene_run;
use vapors_perf::select::matching;
use vapors_perf::spans::PhaseClock;
use vapors_perf::workloads::{settings_for, workloads_at};

fn main() {
    if let Err(error) = run() {
        eprintln!("FAIL perf: {error}");
        std::process::exit(1);
    }
}

#[allow(clippy::too_many_lines)] // the command dispatch, one branch per mode
fn run() -> Result<(), String> {
    let options = options(std::env::args().skip(1))?;
    // a judged run refuses another profile before anything else
    if options.scaling.is_some() {
        build_profile::judged(build_profile::BUILT)?;
    }
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
        first_frame,
        scene_run,
        samples,
    } = options;
    if let Some(kind) = scene_run {
        let open = || {
            let gpu = gpu_with(FEATURES);
            print_adapter(&gpu);
            gpu
        };
        return scene_run::run(open, (kind, samples));
    }
    if let Some(rows) = mount_memory {
        let gpu = gpu_with(FEATURES);
        println!("{}", mount_memory::run_child(&gpu, rows)?.line());
        return Ok(());
    }
    if let Some(rows) = first_frame {
        let open = || {
            let gpu = gpu_with(FEATURES);
            print_adapter(&gpu);
            gpu
        };
        return first_frame_run::run(open, rows);
    }
    if mount {
        return mount_run::run();
    }
    if let Some(changes) = scaling {
        let gpu = gpu_with(FEATURES);
        print_setup(&gpu, settings);
        return scaling_all::run(&gpu, &changes, settings);
    }
    let all = match set {
        WorkloadSet::Main => workloads_at(samples),
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
    println!("Painter samples: {}\n", samples.count());
    let mut timer = Sampler::with_clock(&gpu, clock);
    if set == WorkloadSet::OffPath {
        println!("{OFF_PATH_SETUP}\n");
    }
    for (name, (rows, make)) in chosen {
        let mut workload = make();
        let at = settings_for(rows, settings);
        let samples = timer.sample(workload.as_mut(), at).map_err(|e| {
            let set = if set == WorkloadSet::OffPath {
                " --off-path"
            } else {
                ""
            };
            let (runs, warmup) = (settings.runs, settings.warmup);
            format!("{name}: {e}; reproduce: vapors-perf {runs} {warmup}{set} --only \"{name}\"")
        })?;
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
        "Profile: {}; scale 1; warmup {}; runs {}",
        build_profile::header(),
        settings.warmup,
        settings.runs
    );
    println!("Load average at the start: {}\n", load());
}

fn print_adapter(gpu: &vapors_gpu_harness::Gpu) {
    let info = gpu.adapter.get_info();
    println!(
        "Adapter: {} ({:?}, {} {})",
        info.name, info.backend, info.driver, info.driver_info
    );
    println!("Adapter type: {:?}", info.device_type);
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
