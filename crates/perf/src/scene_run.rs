//! The judged 60 s run of a scene (`SceneKind`: the racer or the
//! one-scene example), from its refusals before the first frame to its
//! verdict (docs/plans/one-scene.md, 3.2): every frame judged, no
//! warmup (D7 (a)), by its frame-to-idle time (D6 (a)).
//!
//! Scene runs are judged against the shipped profile, `release`, on
//! purpose (lead, 2026-11-16): their 16.7 ms budget is about what users
//! run. The judged profile, one codegen unit (`build_profile.rs`), is for
//! the instruction-count growth checks of `--scaling`, where code layout
//! is noise. The header says which profile a run was built with.

use vapors_gpu_harness::Gpu;

use crate::args::SceneProfile;
use crate::driver::RUN_FRAMES;
use crate::run::Workload;
use crate::sampler::{JudgedFrame, Sampler};
use crate::scene_judge::{
    BUDGET, load_average, over_budget, quiet, require_hardware, require_timestamps,
};
use crate::scene_kind::SceneKind;
use crate::scene_report::{over_budget_lines, report};
use crate::scene_size::Resized;
use crate::shader_cache::shader_cache;

/// The revision the harness was built from (`build.rs`).
pub const REVISION: &str = env!("VAPORS_PERF_REVISION");

/// Runs `kind` driven for [`RUN_FRAMES`] frames on the GPU `open` makes,
/// after the shader cache is read, and prints the report: at `profile`'s
/// size when it has one, and without the budget's verdict, on any
/// adapter, when it is `profile_only`.
///
/// # Errors
/// Mesa's shader cache not chosen, a software adapter, a device without
/// timestamps or a busy machine, before the first frame; a frame refused
/// or without a GPU reading, naming it; then every frame over budget and
/// a busy machine at the end.
pub fn run(
    open: impl FnOnce() -> Gpu,
    (kind, samples): (SceneKind, vapors_renderer::Samples),
    profile: SceneProfile,
) -> Result<(), String> {
    let reproduce = reproducing(kind);
    let cache = shader_cache(|name| std::env::var(name).ok()).map_err(&reproduce)?;
    let gpu = open();
    let start = before_frame_zero(&gpu.adapter.get_info(), gpu.device.features(), profile)
        .map_err(&reproduce)?;
    println!(
        "Revision: {REVISION}\nProfile: {}\nShader cache: {}",
        crate::build_profile::scene_header(crate::build_profile::BUILT),
        cache.describe()
    );
    println!(
        "Frames: {RUN_FRAMES}, each held to the frame budget, no warmup; load average at the start {start:.2}; samples {}\n",
        samples.count()
    );
    let mut workload = sized(kind.workload(samples).map_err(&reproduce)?, profile);
    println!("{}", profile_line(workload.size(), profile));
    let run = Sampler::new(&gpu)
        .judged(&mut *workload, RUN_FRAMES)
        .map_err(&reproduce)?;
    println!("{}", start_up_line(run.start_up));
    let end = load_average();
    println!("{}", report(&run.frames, start, end.clone().ok(), kind));
    let judged: &[JudgedFrame] = if profile.profile_only {
        &[]
    } else {
        &run.frames
    };
    verdict(judged, end, kind)
}

/// `workload`, at `profile`'s size when it names one.
fn sized(workload: Box<dyn Workload>, profile: SceneProfile) -> Box<dyn Workload> {
    match profile.size {
        Some(size) => Box::new(Resized::new(workload, size)),
        None => workload,
    }
}

/// The run's size, and whether it is judged.
fn profile_line((width, height): (u32, u32), profile: SceneProfile) -> String {
    let judged = if profile.profile_only {
        "not judged (--profile-only): no budget verdict, any adapter"
    } else {
        "judged against the frame budget"
    };
    format!("Size: {width} x {height}; {judged}\n")
}

/// A refusal of `kind`'s run, with the command that reproduces it.
fn reproducing(kind: SceneKind) -> impl Fn(String) -> String {
    move |error| format!("{error}\nreproduce: {}", kind.reproduce())
}

/// The refusals on the GPU `info` names, with `features`, and on the
/// machine's load: the load at the start when the run may go on.
fn before_frame_zero(
    info: &wgpu::AdapterInfo,
    features: wgpu::Features,
    profile: SceneProfile,
) -> Result<f64, String> {
    let adapter = (info.device_type, info.name.as_str(), info.driver.as_str());
    require_adapter(adapter, features, profile)?;
    quiet("start", load_average())
}

/// The refusals on the adapter (its type, name and driver), with
/// `features`.
fn require_adapter(
    (device_type, name, driver): (wgpu::DeviceType, &str, &str),
    features: wgpu::Features,
    profile: SceneProfile,
) -> Result<(), String> {
    // Relaxed for --profile-only, which reports and judges nothing: a
    // software adapter's times are profiled (13b), never held to the budget.
    if !profile.profile_only {
        require_hardware(device_type, name, driver)?;
    }
    require_timestamps(features, name)
}

/// The start-up's line: `App::start`'s time, before frame 0 and not
/// judged, printed so the time moved out of frame 0 is never hidden.
fn start_up_line(start_up: std::time::Duration) -> String {
    format!(
        "Start-up (App::start, before frame 0, not judged): {:.1} µs\n",
        start_up.as_secs_f64() * 1e6
    )
}

/// The run's failures together: the frames over budget, then the load
/// at the end.
fn verdict(
    frames: &[JudgedFrame],
    end: Result<f64, String>,
    kind: SceneKind,
) -> Result<(), String> {
    let over = over_budget(frames);
    let mut failures = Vec::new();
    if !over.is_empty() {
        failures.push(format!(
            "{} frames over {BUDGET:?} from the start of App::frame to the GPU idle:\n{}",
            over.len(),
            over_budget_lines(&over, kind)
        ));
    }
    if let Err(busy) = quiet("end", end) {
        failures.push(busy);
    }
    if failures.is_empty() {
        return Ok(());
    }
    Err(format!(
        "{}\nreproduce: {}",
        failures.join("\n"),
        kind.reproduce()
    ))
}

#[cfg(test)]
#[path = "scene_run_tests.rs"]
mod tests;
