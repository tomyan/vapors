//! Top-down racing through Vapors platform, GPU rendering and compiled UI.
mod ground_material;
pub use ground_material::ground_material;
mod app;
mod driving;
pub mod model;
pub mod world;
pub mod world_geometry;
pub use app::Racer;
include!(concat!(env!("OUT_DIR"), "/wisp.rs"));

/// Open the racer with the same implementation on desktop and WebGPU.
///
/// # Errors
/// A platform/window/GPU failure, reported with its cause, or the HUD's
/// failure (a refused update, a track render error) that ended the loop.
pub fn run() -> Result<(), String> {
    run_with(None)
}

/// As [`run`], with the HUD and the world drawn at `samples` and their
/// GPU time written to the demo bar's line once a second, when given (a
/// run by hand, `docs/plans/flat-scene-f7.md`, F7b); at 4 samples
/// otherwise (`Racer::new`).
///
/// # Errors
/// As [`run`].
pub fn run_with(readout: Option<vapors_renderer::Samples>) -> Result<(), String> {
    vapors_platform::app_draws_open();
    vapors_platform::hide_status_line();
    vapors_dom::set_error_report(
        "vapors_platform::report_error",
        vapors_platform::report_error,
    );
    let samples = readout.unwrap_or(vapors_renderer::Samples::Four);
    let mut racer = app::Racer::with_samples(samples);
    if readout.is_some() {
        racer.show_gpu_time();
    }
    let failure = racer.failure();
    vapors_platform::run(
        vapors_platform::WindowOptions {
            app_name: "vapors racer".into(),
            title: "Vapors / Circuit".into(),
            width: 1100,
            height: 720,
            decorations: true,
            app_id: None,
        },
        racer,
    )
    .map_err(|error| error.to_string())?;
    failure.get().map_or(Ok(()), Err)
}

#[cfg(test)]
mod app_tests;

#[cfg(test)]
mod component_input_tests;

// at the crate root, so each test's path starts `gpu_` as the gpu tier's filter needs
#[cfg(test)]
mod gpu_ground_material_target;
#[cfg(test)]
mod gpu_ground_material_tests;
#[cfg(test)]
mod gpu_world_compare;
#[cfg(test)]
mod gpu_world_golden;
#[cfg(test)]
mod gpu_world_rig;
#[cfg(test)]
mod gpu_world_tests;

#[cfg(test)]
mod gpu_track_idle_tests;
#[cfg(test)]
mod gpu_track_lifecycle_tests;
#[cfg(test)]
mod gpu_track_resize_tests;
#[cfg(test)]
mod gpu_track_scale_tests;
#[cfg(test)]
mod gpu_track_tests;

#[cfg(test)]
mod gpu_frames;
#[cfg(test)]
mod gpu_hud_iu5_probe;
#[cfg(test)]
mod gpu_hud_iu5_tests;
#[cfg(test)]
mod hud_320_tests;
