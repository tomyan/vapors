//! The judged 60 s run of a scene (`SceneKind`: the racer or the
//! one-scene example), from its refusals before the first frame to its
//! verdict (docs/plans/one-scene.md, 3.2): every frame judged, no
//! warmup (D7 (a)), by its frame-to-idle time (D6 (a)).

use vapors_gpu_harness::Gpu;

use crate::driver::RUN_FRAMES;
use crate::sampler::{JudgedFrame, Sampler};
use crate::scene_judge::{
    BUDGET, load_average, over_budget, quiet, require_hardware, require_timestamps,
};
use crate::scene_kind::SceneKind;
use crate::scene_report::{over_budget_lines, report};
use crate::shader_cache::shader_cache;

/// The revision the harness was built from (`build.rs`).
pub const REVISION: &str = env!("VAPORS_PERF_REVISION");

/// Runs `kind` driven for [`RUN_FRAMES`] frames on the GPU `open` makes,
/// after the shader cache is read, and prints the report.
///
/// # Errors
/// Mesa's shader cache not chosen, a software adapter, a device without
/// timestamps or a busy machine, before the first frame; a frame refused
/// or without a GPU reading, naming it; then every frame over budget and
/// a busy machine at the end.
pub fn run(open: impl FnOnce() -> Gpu, kind: SceneKind) -> Result<(), String> {
    let cache = shader_cache(|name| std::env::var(name).ok())?;
    let gpu = open();
    let info = gpu.adapter.get_info();
    require_hardware(info.device_type, &info.name, &info.driver)?;
    require_timestamps(gpu.device.features(), &info.name)?;
    let start = quiet("start", load_average())?;
    println!("Revision: {REVISION}\nShader cache: {}", cache.describe());
    println!(
        "Frames: {RUN_FRAMES}, every one judged, no warmup; load average at the start {start:.2}\n"
    );
    let reproduce = |error| format!("{error}\nreproduce: {}", kind.reproduce());
    let mut workload = kind.workload().map_err(reproduce)?;
    let run = Sampler::new(&gpu)
        .judged(&mut *workload, RUN_FRAMES)
        .map_err(reproduce)?;
    println!("{}", start_up_line(run.start_up));
    let end = load_average();
    println!("{}", report(&run.frames, start, end.clone().ok(), kind));
    verdict(&run.frames, end, kind)
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
