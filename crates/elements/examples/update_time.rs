//! Times `Document::update` on a 1,000-element tree whose one counter
//! text node changes before each update, as the `stress` demo does:
//! `cargo run --release -p vapors-elements --example update_time`.
//! Prints the median and the 95th percentile of 200 updates after 20
//! warm-up ones, at 640x400 and 1x. CPU only: no GPU work is timed.

use std::time::{Duration, Instant};

use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};
use vapors_text::FontSystem;

const ROWS: usize = 100;
const CELLS: usize = 9;

fn main() {
    let (mut doc, counter) = stress_tree();
    let viewport = Viewport {
        width: 640.0,
        height: 400.0,
        scale: 1.0,
    };
    let mut times = Vec::new();
    for tick in 0..220 {
        doc.set_text(counter, &format!("tick {tick}"));
        let started = Instant::now();
        if let Err(errors) = doc.update(viewport) {
            eprintln!("FAIL update_time: the update was refused: {errors:?}");
            std::process::exit(1);
        }
        if tick >= 20 {
            times.push(started.elapsed());
        }
    }
    times.sort();
    let at = |percent: usize| times[(times.len() - 1) * percent / 100];
    println!(
        "update_time: {} elements, {} updates: median {:?}, p95 {:?}, max {:?}",
        elements(),
        times.len(),
        at(50),
        at(95),
        times.last().copied().unwrap_or(Duration::ZERO)
    );
}

fn elements() -> usize {
    1 + ROWS * (1 + CELLS)
}

/// A column of 100 flex rows of 9 cells with text, and a counter: 1,001
/// elements under body.
fn stress_tree() -> (Document, NodeId) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet(
        "stress.css",
        "body { margin: 0; font-size: 12px; } .row { display: flex; gap: 4px; } .cell { width: 60px; background-color: #eef; }",
    )
    .expect("parses");
    let body = doc.body();
    let column = doc.create_element("div");
    doc.append(body, column);
    let counter = doc.create_text("tick 0");
    doc.append(column, counter);
    for row in 0..ROWS {
        let r = doc.create_element("div");
        doc.set_attribute(r, "class", "row");
        doc.append(column, r);
        for cell in 0..CELLS {
            let c = doc.create_element("div");
            doc.set_attribute(c, "class", "cell");
            let text = doc.create_text(&format!("{row}.{cell}"));
            doc.append(c, text);
            doc.append(r, c);
        }
    }
    (doc, counter)
}
