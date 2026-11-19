//! The demos page. `?demo=<name>` picks what it shows: an elements demo
//! from `vapors-demos` (F2c), `racer`, `one-scene` (HTML over a 3D scene
//! styled by CSS, `vapors-one-scene`), or `image`, the image app (F0: the
//! gradient fixture fitted to the canvas; a tap, click or `o` opens a
//! file), or `wake`, the waker's check (`wake.rs`). With no `?demo=` it
//! shows the index, which lists them. The
//! same binary runs natively, which is how `cargo clippy --workspace`
//! checks it; natively there is no query, so it shows the index.

mod apps;
#[cfg(target_family = "wasm")]
mod bar;
mod bar_query;
mod samples;
mod wake;

use vapors_app::ElementsApp;
use vapors_platform::{
    WindowOptions, app_draws_open, hide_status_line, query_parameter, query_parameters,
    report_error, run,
};

fn main() {
    if let Err(message) = start() {
        report_error(&message);
        #[cfg(not(target_family = "wasm"))]
        std::process::exit(1);
    }
}

fn start() -> Result<(), String> {
    let name = query_parameter("demo").unwrap_or_else(|| vapors_demos::INDEX.to_owned());
    let options = WindowOptions {
        app_name: "vapors".into(),
        title: format!("vapors demo: {name}"),
        width: 640,
        height: 400,
        decorations: true,
        app_id: None,
    };
    // the bar stepping through the listed demos; none for the index,
    // nor with ?bar=0 (bar_query.rs)
    let shows_bar = bar_query::bar_shown(&query_parameters("bar"))?;
    #[cfg(target_family = "wasm")]
    if shows_bar {
        bar::show(&name)?;
    }
    // natively there is no page, so no bar
    #[cfg(not(target_family = "wasm"))]
    let _ = shows_bar;
    // read once, here, so each run below takes it or refuses it
    let samples = samples::samples_of(&query_parameters("samples"))?;
    if let Some(app) = apps::named(&name) {
        return apps::run_app(app, options, samples);
    }
    if name == "wake" {
        samples::none_for("wake", samples)?;
        return run(options, wake::WakeApp).map_err(|e| e.to_string());
    }
    run_elements(&name, options, samples)
}

/// The elements demo `name`, or the index, in a window from `options`,
/// at `samples` with its GPU time shown when they are given.
///
/// # Errors
/// An unknown name, a bad `ticks=`, or the window failing.
fn run_elements(
    name: &str,
    options: WindowOptions,
    samples: Option<vapors_renderer::Samples>,
) -> Result<(), String> {
    // `own-open` is `hello` with the page's status line left as it is: the
    // page check's case for an app that draws its own Open and keeps `#status`.
    // A test hook for scripts/check-page-contract.sh only; like `wake`, it is
    // not in `vapors_demos`, so no demo list or error message names it.
    let keeps_status = name == "own-open";
    let demo = vapors_demos::find(if keeps_status { "hello" } else { name })?;
    // An elements demo draws its own controls; the page's would cover them.
    app_draws_open();
    if !keeps_status {
        hide_status_line();
    }
    let shown = vapors_demos::document(&demo);
    // `ticks=N` stops a demo that changes every frame after N frames, so
    // a headless page goes idle for its screenshot (the page contract).
    let per_frame = match vapors_demos::frame_limit(query_parameter("ticks").as_deref())? {
        Some(frames) => shown
            .per_frame
            .map(|hook| vapors_demos::limited(hook, frames)),
        None => shown.per_frame,
    };
    let mut app = ElementsApp::new(shown.document).with_per_frame(per_frame);
    if let Some(samples) = samples {
        app.set_samples(samples);
        app.show_gpu_time();
    }
    // the page's hidden DOM mirror of the a11y tree (R6)
    app.mirror_a11y(true).map_err(|e| e.to_string())?;
    run(options, app).map_err(|e| e.to_string())
}
