//! The 60 s run of the racer, from its refusals before the first frame
//! to its verdict (docs/plans/one-scene.md, 3.2): every frame judged,
//! no warmup (D7 (a)), by its frame-to-idle time (D6 (a)).

use vapors_gpu_harness::Gpu;

use crate::driver::RUN_FRAMES;
use crate::racer_driven::DrivenRacer;
use crate::sampler::{JudgedFrame, Sampler};
use crate::scene_judge::{BUDGET, REPRODUCE, load_average, over_budget, quiet, require_timestamps};
use crate::scene_report::{over_budget_lines, report};

/// Runs the driven racer for [`RUN_FRAMES`] frames on `gpu` and prints
/// the report.
///
/// # Errors
/// A device without timestamps or a busy machine, before the first
/// frame; a frame refused or without a GPU reading, naming it; then
/// every frame over budget and a busy machine at the end.
pub fn run(gpu: &Gpu) -> Result<(), String> {
    require_timestamps(gpu.device.features(), &gpu.adapter.get_info().name)?;
    let start = quiet("start", load_average())?;
    println!(
        "Frames: {RUN_FRAMES}, every one judged, no warmup; load average at the start {start:.2}\n"
    );
    let frames = Sampler::new(gpu).judged(&mut DrivenRacer::new(), RUN_FRAMES)?;
    let end = load_average();
    println!("{}", report(&frames, start, end.clone().ok()));
    verdict(&frames, end)
}

/// The run's failures together: the frames over budget, then the load
/// at the end.
fn verdict(frames: &[JudgedFrame], end: Result<f64, String>) -> Result<(), String> {
    let over = over_budget(frames);
    let mut failures = Vec::new();
    if !over.is_empty() {
        failures.push(format!(
            "{} frames over {BUDGET:?} from the start of App::frame to the GPU idle:\n{}",
            over.len(),
            over_budget_lines(&over)
        ));
    }
    if let Err(busy) = quiet("end", end) {
        failures.push(busy);
    }
    if failures.is_empty() {
        return Ok(());
    }
    Err(format!("{}\nreproduce: {REPRODUCE}", failures.join("\n")))
}

#[cfg(test)]
#[path = "scene_run_tests.rs"]
mod tests;
