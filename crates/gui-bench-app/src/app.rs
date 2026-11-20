//! The window of the `app` scenario, and the refusal for what a later
//! slice builds.

use crate::bench_app::{BenchApp, BenchAppProps};
use crate::cli::Scenario;
use crate::dashboard::WINDOW;
use std::{cell::RefCell, rc::Rc};
use vapors_app::ElementsApp;
use vapors_elements::Document;
use vapors_platform::WindowOptions;
use vapors_signals::Signal;
use vapors_text::FontSystem;

/// Opens the window for `scenario`; the `list` scenario is refused, naming
/// the slice and the command that works. The `app` scenario's `--rows`
/// count sizes the orders and messages pages, which s1b and s2 build: the
/// sidebar shows none, so the count has nothing to size yet.
pub fn run(scenario: Scenario) -> Result<(), String> {
    match scenario {
        Scenario::List { .. } => Err(not_built("the `list` scenario")),
        Scenario::App { .. } => open(scenario.title()),
    }
}

/// The refusal for something slice s3 builds.
pub fn not_built(what: &str) -> String {
    format!(
        "{what} is not built yet: s3 (docs/decisions/2026-11-19-gui-bench.md); run `vapors-bench --scenario app` for the overview page"
    )
}

/// The dashboard's document, mounted, with the active nav item.
pub struct Mounted {
    pub document: Rc<RefCell<Document>>,
    #[allow(dead_code)] // read by the tests, which click the nav items
    pub active: Signal<usize>,
    _root: BenchApp,
}

/// Mounts the dashboard into a fresh document.
pub fn mount() -> Mounted {
    let document = Rc::new(RefCell::new(Document::new(FontSystem::new())));
    let body = document.borrow().body();
    let root = BenchApp::mount(&document, body, BenchAppProps::default());
    Mounted {
        document,
        active: root.active,
        _root: root,
    }
}

fn open(title: &str) -> Result<(), String> {
    let mounted = mount();
    let app = ElementsApp::new(Rc::clone(&mounted.document));
    let failure = app.failure();
    vapors_platform::run(options(title), app).map_err(|error| error.to_string())?;
    failure.get().map_or(Ok(()), Err)
}

/// The window the spec asks for: 1280x800, borderless, the app id the
/// runner looks for. The platform refuses to open it where it cannot
/// honour either, so the runner never waits for a window that lacks them.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // WINDOW is whole and positive
pub fn options(title: &str) -> WindowOptions {
    WindowOptions {
        app_name: "vapors-bench".into(),
        title: title.into(),
        width: WINDOW.0 as u32,
        height: WINDOW.1 as u32,
        decorations: false,
        app_id: Some("gui-bench.vapors".into()),
    }
}
