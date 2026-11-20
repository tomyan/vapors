//! The window of the `app` scenario, and the refusal for the `list`
//! scenario, which a later slice builds.

use crate::bench_app::{BenchApp, BenchAppProps};
use crate::cli::Scenario;
use crate::controls::Controls;
use crate::dashboard::{KPIS, Order, WINDOW, arranged, visible};
use crate::order_parts::PILLS;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use vapors_app::ElementsApp;
use vapors_elements::Document;
use vapors_platform::WindowOptions;
use vapors_signals::Signal;
use vapors_text::FontSystem;

/// Opens the window for `scenario`; the `list` scenario is refused, naming
/// the slice and the command that works. `--rows` sizes the messages feed.
pub fn run(scenario: Scenario) -> Result<(), String> {
    match scenario {
        Scenario::List { .. } => Err(not_built("the `list` scenario")),
        Scenario::App { count } => open(scenario.title(), count),
    }
}

/// The refusal for something a later slice builds.
pub fn not_built(what: &str) -> String {
    format!(
        "{what} is not built yet: s3b (docs/decisions/2026-11-19-gui-bench.md); run `vapors-bench --scenario app` for the dashboard"
    )
}

/// The dashboard's document, mounted, with the active nav item.
pub struct Mounted {
    pub document: Rc<RefCell<Document>>,
    #[allow(dead_code)] // read by the tests, which click the nav items
    pub active: Signal<usize>,
    #[allow(dead_code)] // read by the tests, which type into the search input
    pub search: Signal<String>,
    #[allow(dead_code)] // read by the tests, which read the chart's and the filters' state
    pub controls: Controls,
    _root: BenchApp,
}

/// Mounts the dashboard into a fresh document, with `rows` messages.
pub fn mount(rows: usize) -> Mounted {
    let document = Rc::new(RefCell::new(Document::new(FontSystem::new())));
    let body = document.borrow().body();
    let controls = Controls::new();
    let root = BenchApp::mount(
        &document,
        body,
        BenchAppProps {
            rows,
            controls: controls.into(),
        },
    );
    Mounted {
        document,
        active: root.active,
        search: root.search,
        controls,
        _root: root,
    }
}

/// The heights of metric card `k`'s sparkline bars, in whole px, by bar.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // the spec's heights are whole, 14 to 44
pub fn spark_heights(k: usize) -> Vec<(usize, u32)> {
    let heights = KPIS[k].spark.iter().map(|height| *height as u32);
    heights.enumerate().collect()
}

/// The orders the table shows, as indices into `base`: the visible ones
/// (`visible`), arranged by `arranged`. An order's index is its place in
/// `base`, found by its number, which must be unique (a repeat would give
/// two rows one key).
pub fn order_ids(base: &[Order], shown: [bool; PILLS], search: &str, sorted: bool) -> Vec<usize> {
    let mut place: HashMap<u32, usize> = HashMap::new();
    for (i, o) in base.iter().enumerate() {
        let twin = place.insert(o.number, i);
        assert!(
            twin.is_none(),
            "orders {twin:?} and {i} share the number {}: row keys must be unique (cargo test -p gui-bench-app orders_behaviour)",
            o.number
        );
    }
    let kept: Vec<Order> = base
        .iter()
        .filter(|o| visible(o, &shown, search))
        .cloned()
        .collect();
    arranged(kept, sorted)
        .iter()
        .map(|o| place[&o.number])
        .collect()
}

fn open(title: &str, rows: usize) -> Result<(), String> {
    let mounted = mount(rows);
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
