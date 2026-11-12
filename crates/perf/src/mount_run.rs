//! `--mount`: the Mount workload at each size of [`SIZES`], tabled.

use vapors_gpu_harness::Gpu;

use crate::instructions_check::{checked_counter, samples_counted};
use crate::mount::Mount;
use crate::mount_memory::{in_processes, memory_table};
use crate::mount_report::MOUNT_PHASES;
use crate::mount_report::{Measured, dominant, instructions_table};
use crate::report::times_section;
use crate::run::{Samples, Settings};
use crate::sampler::Sampler;
use crate::scaling::SIZES;
use crate::scaling_run::at_size;
use crate::spans::PhaseClock;
use crate::stats::median_count;

/// Mounts kept per size: a 10,000-row mount and first frame take
/// seconds, and their counts repeat within a few percent in fewer.
pub const MAX_RUNS: usize = 9;

/// Mounts run before the kept ones: the first page of a size, whose
/// resident set is the one reported.
const WARMUP: usize = 1;

/// Runs the workload at every size, prints the tables.
///
/// # Errors
/// Counts that cannot be trusted, a resident set that cannot be read,
/// or a mount that failed.
pub fn run(gpu: &Gpu, settings: Settings) -> Result<(), String> {
    let settings = Settings {
        warmup: WARMUP,
        runs: settings.runs.min(MAX_RUNS),
    };
    println!(
        "Mount: warmup {} (the first page of each size), runs {} (at most {MAX_RUNS}); the profile line above is the full run's, not this one's\n",
        settings.warmup, settings.runs
    );
    let mut timer = Sampler::with_clock(gpu, PhaseClock::with_instructions(checked_counter()?));
    let mut measured = Vec::new();
    for rows in SIZES {
        let mut workload = Mount::new(rows, checked_counter()?);
        let samples = timer
            .sample(&mut workload, settings)
            .map_err(|e| format!("{rows} rows, Mount: {e}"))?;
        println!("{}", mount_times(rows, &workload, &samples, settings)?);
        measured.push(measure(rows, &workload, &samples, settings)?);
    }
    println!("### Mount and first frame, median instructions\n");
    println!("{}", instructions_table(&measured)?);
    println!("### Resident set, first mount and frame of each size, one process per size\n");
    println!("{}", memory_table(&in_processes(&SIZES)?));
    println!("{}", dominant(&measured)?);
    Ok(())
}

fn mount_times(
    rows: usize,
    workload: &Mount,
    samples: &Samples,
    settings: Settings,
) -> Result<String, String> {
    let kept = kept_mounts(workload, settings)?;
    let times: Vec<_> = kept.iter().map(|m| m.time).collect();
    let median = crate::stats::Distribution::of(&times)
        .ok_or("no mounts were kept")?
        .median;
    Ok(format!(
        "Mount, {rows} rows: median mount {:.1} µs over {} kept mounts.\n\n{}",
        median.as_secs_f64() * 1e6,
        kept.len(),
        times_section(&format!("{rows} rows, first frame"), samples, &MOUNT_PHASES)
    ))
}

fn kept_mounts(workload: &Mount, settings: Settings) -> Result<&[crate::mount::Mounted], String> {
    let kept = workload
        .mounted
        .get(settings.warmup..settings.warmup + settings.runs);
    kept.ok_or_else(|| {
        format!(
            "{} mounts were measured; {} were needed",
            workload.mounted.len(),
            settings.warmup + settings.runs
        )
    })
}

fn measure(
    rows: usize,
    workload: &Mount,
    samples: &Samples,
    settings: Settings,
) -> Result<Measured, String> {
    samples_counted(&format!("{rows} rows, Mount"), samples)?;
    let mut instructions = at_size(rows, samples, &MOUNT_PHASES)?.phases;
    let kept = kept_mounts(workload, settings)?;
    let counts: Vec<u64> = kept.iter().map(|m| m.instructions).collect();
    let mount = median_count(&counts).ok_or("no mounts were kept")?;
    instructions.insert("mount".into(), mount);
    Ok(Measured { rows, instructions })
}
