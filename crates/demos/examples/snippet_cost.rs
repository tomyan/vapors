//! What snippets cost (1-R1): the `cards` demo, whose cards are a Card
//! component given its contents as a children snippet built through
//! `dyn Dom`, against `cards_flat`, the same tree written out in place.
//! `cargo run --release -p vapors-demos --example snippet_cost`. Prints
//! the median of 50 mounts with their first update, and of 200 updates
//! after a signal every card shows changes (after 20 warm-up ones), at
//! 640x400 and 1x. CPU only: no GPU work is timed.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use vapors_demos::cards::{Cards, CardsProps};
use vapors_demos::cards_flat::{CardsFlat, CardsFlatProps};
use vapors_demos::register_thumbnails;
use vapors_elements::{Document, Viewport};
use vapors_signals::Signal;
use vapors_text::FontSystem;

const VIEWPORT: Viewport = Viewport {
    width: 640.0,
    height: 400.0,
    scale: 1.0,
};

fn main() {
    let snippets = measure("cards (snippets)", |doc, body| {
        Cards::mount(doc, body, CardsProps {}).tick
    });
    let flat = measure("cards_flat (no snippets)", |doc, body| {
        CardsFlat::mount(doc, body, CardsFlatProps {}).tick
    });
    for (name, mount, update) in [snippets, flat] {
        println!(
            "snippet_cost: {name}: mount and first update median {mount:?}; update median {update:?}"
        );
    }
}

/// Mounts with `mount`, which returns the signal every card shows; the
/// medians of mounting with the first update, and of later updates.
fn measure(
    name: &'static str,
    mount: impl Fn(&Rc<RefCell<Document>>, vapors_dom::NodeId) -> Signal<u32>,
) -> (&'static str, Duration, Duration) {
    let mut mounts = Vec::new();
    for _ in 0..50 {
        let started = Instant::now();
        let (doc, _tick) = mounted(&mount);
        update(&doc);
        mounts.push(started.elapsed());
    }
    let (doc, tick) = mounted(&mount);
    let mut updates = Vec::new();
    for step in 0..220 {
        tick.set(step);
        let started = Instant::now();
        update(&doc);
        if step >= 20 {
            updates.push(started.elapsed());
        }
    }
    (name, median(mounts), median(updates))
}

fn mounted(
    mount: &impl Fn(&Rc<RefCell<Document>>, vapors_dom::NodeId) -> Signal<u32>,
) -> (Rc<RefCell<Document>>, Signal<u32>) {
    let doc = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    register_thumbnails(&mut doc.borrow_mut());
    let body = doc.borrow().body();
    let tick = mount(&doc, body);
    (doc, tick)
}

fn update(doc: &Rc<RefCell<Document>>) {
    if let Err(errors) = doc.borrow_mut().update(VIEWPORT) {
        eprintln!("FAIL snippet_cost: the update was refused: {errors:?}");
        std::process::exit(1);
    }
}

fn median(mut times: Vec<Duration>) -> Duration {
    times.sort();
    times[times.len() / 2]
}
