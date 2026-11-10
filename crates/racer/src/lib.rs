//! Top-down racing through Vapors platform, GPU rendering and compiled UI.
mod app;
mod driving;
mod gpu;
pub mod model;
include!(concat!(env!("OUT_DIR"), "/wisp.rs"));

/// Open the racer with the same implementation on desktop and WebGPU.
///
/// # Errors
/// A platform/window/GPU failure, reported with its cause, or the HUD's
/// failure (a refused update, a track render error) that ended the loop.
pub fn run() -> Result<(), String> {
    vapors_platform::app_draws_open();
    vapors_platform::hide_status_line();
    vapors_dom::set_error_report(
        "vapors_platform::report_error",
        vapors_platform::report_error,
    );
    let racer = app::Racer::new();
    let failure = racer.failure();
    vapors_platform::run(
        vapors_platform::WindowOptions {
            app_name: "vapors racer".into(),
            title: "Vapors / Circuit".into(),
            width: 1100,
            height: 720,
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
mod gpu_track_unmount_order_tests;
#[cfg(test)]
mod gpu_track_unmount_tests;

#[cfg(test)]
mod gpu_frames;
#[cfg(test)]
mod gpu_hud_iu5_probe;
#[cfg(test)]
mod gpu_hud_iu5_tests;
