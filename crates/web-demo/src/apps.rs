//! The apps the page runs itself, by their `?demo=` name: the one
//! answer to which `Kind::App` entries of `vapors_demos::DEMOS` `start()`
//! can run, held to that list by the tests below.

use vapors_canvas::{ImageApp, decode_png};
use vapors_platform::{WindowOptions, query_parameter, run};
use vapors_scene::Color;

const GRADIENT: &[u8] = include_bytes!("../../platform/examples/fixtures/gradient.png");

const BACKGROUND: Color = Color {
    r: 40.0 / 255.0,
    g: 40.0 / 255.0,
    b: 40.0 / 255.0,
    a: 1.0,
};

/// One app the page runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppDemo {
    /// `vapors-racer`.
    Racer,
    /// HTML over a 3D scene styled by CSS, `vapors-one-scene`.
    OneScene,
    /// The image app: the gradient fixture fitted to the canvas; a tap,
    /// click or `o` opens a file.
    Image,
}

/// Every app the page runs, by name.
pub const APPS: [(&str, AppDemo); 3] = [
    ("racer", AppDemo::Racer),
    ("one-scene", AppDemo::OneScene),
    ("image", AppDemo::Image),
];

/// The app named `name`, if the page runs one by that name.
/// The page's `?samples=`, when given (`samples.rs`).
///
/// # Errors
/// As [`crate::samples::samples_of`].
pub fn page_samples() -> Result<Option<vapors_renderer::Samples>, String> {
    crate::samples::samples_of(query_parameter("samples").as_deref())
}

pub fn named(name: &str) -> Option<AppDemo> {
    APPS.iter()
        .find(|(each, _)| *each == name)
        .map(|(_, app)| *app)
}

/// Runs `app` in a window made from `options`.
///
/// # Errors
/// The app's own start or run failing, or a bad `ticks=`.
pub fn run_app(app: AppDemo, options: WindowOptions) -> Result<(), String> {
    match app {
        AppDemo::Racer => vapors_racer::run_with(page_samples()?),
        AppDemo::OneScene => {
            let ticks = vapors_demos::frame_limit(query_parameter("ticks").as_deref())?;
            vapors_one_scene::run(ticks.map(|frames| frames as u64), page_samples()?)
        }
        AppDemo::Image => {
            let image =
                decode_png(GRADIENT).map_err(|message| format!("gradient.png: {message}"))?;
            let app = ImageApp::showing("gradient.png", image, BACKGROUND);
            run(options, app).map_err(|e| e.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use vapors_demos::{DEMOS, Kind, listed};

    #[test]
    fn every_listed_app_is_one_the_page_runs() {
        for demo in DEMOS.iter().filter(|demo| demo.kind == Kind::App) {
            assert!(
                super::named(demo.name).is_some(),
                "{} is listed as an app; add it to APPS",
                demo.name
            );
        }
    }

    #[test]
    fn every_app_the_page_runs_is_listed_as_an_app() {
        for (name, _) in super::APPS {
            let kind = listed(name).map(|demo| demo.kind);
            assert_eq!(
                kind,
                Some(Kind::App),
                "{name} is run but not listed as an app in DEMOS"
            );
        }
    }
}
