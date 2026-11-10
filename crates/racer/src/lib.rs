//! Top-down racing through Vapors platform, GPU rendering and compiled UI.
mod app;
mod driving;
mod gpu;
pub mod model;
include!(concat!(env!("OUT_DIR"), "/wisp.rs"));

/// Open the racer with the same implementation on desktop and WebGPU.
///
/// # Errors
/// A platform/window/GPU failure, reported with its cause.
pub fn run() -> Result<(), String> {
    vapors_platform::hide_page_controls();
    vapors_dom::set_error_report(
        "vapors_platform::report_error",
        vapors_platform::report_error,
    );
    vapors_platform::run(
        vapors_platform::WindowOptions {
            app_name: "vapors racer".into(),
            title: "Vapors / Circuit".into(),
            width: 1100,
            height: 720,
        },
        app::Racer::new(),
    )
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod app_tests;

#[cfg(test)]
mod component_input_tests;
