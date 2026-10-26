//! What snippets cost (1-R1): the `cards` demo, whose cards are a Card
//! component given its contents as a children snippet built through
//! `dyn Dom`, against `cards_flat`, the same tree written out in place.
//! `cargo run --release -p vapors-demos --example snippet_cost`.
//!
//! Two windows, each the median of 100 runs on a fresh document whose
//! fonts and thumbnails are loaded before the timer starts:
//! - mount: the component's `mount` alone, where snippet closures are
//!   built and rendered;
//! - first update: the `Document::update` after it, which styles and
//!   lays out the tree.
//!
//! Later updates are not compared: after mount the two trees are the
//! same and no snippet code runs on an update, so they could not show a
//! snippet's cost. CPU only, at 640x400 and 1x. Read the numbers only on
//! a quiet box; the load average is printed with them.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use vapors_demos::cards::{Cards, CardsProps};
use vapors_demos::cards_flat::{CardsFlat, CardsFlatProps};
use vapors_demos::register_thumbnails;
use vapors_dom::NodeId;
use vapors_elements::{Document, Viewport};
use vapors_text::FontSystem;

const VIEWPORT: Viewport = Viewport {
    width: 640.0,
    height: 400.0,
    scale: 1.0,
};
const RUNS: usize = 100;

type Shared = Rc<RefCell<Document>>;

fn main() {
    println!("snippet_cost: load average {}", load_average());
    let snippets = measure(|doc, body| drop(Cards::mount(doc, body, CardsProps {})));
    let flat = measure(|doc, body| drop(CardsFlat::mount(doc, body, CardsFlatProps {})));
    for (name, (mount, update)) in [
        ("cards (snippets)", snippets),
        ("cards_flat (no snippets)", flat),
    ] {
        println!("snippet_cost: {name}: mount median {mount:?}; first update median {update:?}");
    }
}

/// The medians of mounting with `mount`, and of the first update after.
fn measure(mount: impl Fn(&Shared, NodeId)) -> (Duration, Duration) {
    let (mut mounts, mut updates) = (Vec::new(), Vec::new());
    for _ in 0..RUNS {
        let doc = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
        register_thumbnails(&mut doc.borrow_mut());
        let body = doc.borrow().body();
        let started = Instant::now();
        mount(&doc, body);
        mounts.push(started.elapsed());
        let started = Instant::now();
        update(&doc);
        updates.push(started.elapsed());
    }
    (median(mounts), median(updates))
}

fn update(doc: &Shared) {
    if let Err(errors) = doc.borrow_mut().update(VIEWPORT) {
        eprintln!("FAIL snippet_cost: the update was refused: {errors:?}");
        std::process::exit(1);
    }
}

fn median(mut times: Vec<Duration>) -> Duration {
    times.sort();
    times[times.len() / 2]
}

/// The one-minute load average, or why it could not be read.
fn load_average() -> String {
    std::fs::read_to_string("/proc/loadavg").map_or_else(
        |e| format!("unknown (/proc/loadavg: {e})"),
        |text| {
            text.split_whitespace()
                .next()
                .unwrap_or("unknown")
                .to_owned()
        },
    )
}
