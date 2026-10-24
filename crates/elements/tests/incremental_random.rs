//! A seeded randomized differential test of incremental updates (F2c
//! S7 review). Each step makes one random change and updates; then the
//! document forgets everything kept between updates and updates from
//! scratch. Every element's border box, computed style, the painted
//! display list with each item's clip, and what a grid of points hits
//! must be the same both ways. CI runs `SEEDS` seeds; set
//! `VAPORS_INCREMENTAL_SEEDS=<n>` to run more. A failure names the
//! seed, the step and the change.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::changes::{Random, change, page};
use support::viewport;
use vapors_css::cascade::ComputedStyle;
use vapors_dom::NodeId;
use vapors_elements::{BoxRect, Document, Viewport};
use vapors_scene::{Item, Rect};

const SEEDS: u64 = 6;
const STEPS: usize = 40;

/// What is compared: each element's border box and computed style, in
/// tree order; the display list with each text run as its text, and
/// each item's clip; and the element hit at every `GRID` CSS px.
#[derive(Debug, PartialEq)]
struct Snapshot {
    boxes: Vec<(NodeId, Option<BoxRect>)>,
    styles: Vec<Option<ComputedStyle>>,
    items: Vec<String>,
    clips: Vec<Option<Rect>>,
    hits: Vec<Option<NodeId>>,
}

/// The spacing of the hit grid, CSS px.
const GRID: usize = 10;

/// The settled frame: an update can move content under the pointer,
/// which it notices after painting (it sets the hover flags and asks
/// for another frame), so this updates until no change is pending; at
/// most `SETTLE` times, or the test fails.
fn snapshot(doc: &Rc<RefCell<Document>>, viewport: Viewport) -> Snapshot {
    let mut doc = doc.borrow_mut();
    doc.take_changed();
    for _ in 0..SETTLE {
        doc.update(viewport)
            .unwrap_or_else(|errors| panic!("refused: {errors:#?}"));
        if !doc.take_changed() {
            return settled(&mut doc, viewport);
        }
    }
    panic!("the document still changed after {SETTLE} updates");
}

/// Updates that a frame may take to settle: one, and one more for a
/// hover the first moved.
const SETTLE: usize = 3;

fn settled(doc: &mut Document, viewport: Viewport) -> Snapshot {
    let nodes = doc.elements();
    let painted = doc
        .update(viewport)
        .unwrap_or_else(|errors| panic!("refused: {errors:#?}"));
    let boxes = nodes.iter().map(|n| (*n, painted.border_box(*n))).collect();
    let items = painted
        .scene()
        .items()
        .iter()
        .map(|item| match item {
            Item::Text { text, origin } => {
                format!("text {:?} at {origin:?}", painted.text(*text).text())
            }
            other => format!("{other:?}"),
        })
        .collect();
    let clips = painted.scene().clips().to_vec();
    let hits = grid(viewport).map(|point| painted.hit(point)).collect();
    let styles = nodes
        .iter()
        .map(|n| doc.computed_style(*n).cloned())
        .collect();
    Snapshot {
        boxes,
        styles,
        items,
        clips,
        hits,
    }
}

/// Points `GRID` CSS px apart over `viewport`.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)] // viewports are small and whole
fn grid(viewport: Viewport) -> impl Iterator<Item = [f32; 2]> {
    let (width, height) = (viewport.width as usize, viewport.height as usize);
    (0..height)
        .step_by(GRID)
        .flat_map(move |y| (0..width).step_by(GRID).map(move |x| [x as f32, y as f32]))
}

fn run(seed: u64) {
    let mut random = Random(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let doc = page(&mut random);
    let mut viewport = viewport(640.0, 400.0);
    snapshot(&doc, viewport);
    for step in 0..STEPS {
        let what = change(&doc, &mut random, &mut viewport, step);
        let kept = snapshot(&doc, viewport);
        doc.borrow_mut().forget_kept();
        let fresh = snapshot(&doc, viewport);
        assert_eq!(
            kept, fresh,
            "seed {seed}, step {step}: {what}; reproduce: VAPORS_INCREMENTAL_SEED={seed}"
        );
    }
}

#[test]
fn incremental_updates_equal_updates_from_scratch_for_random_changes() {
    if let Some(seed) = std::env::var("VAPORS_INCREMENTAL_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
    {
        return run(seed);
    }
    let seeds = std::env::var("VAPORS_INCREMENTAL_SEEDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(SEEDS);
    for seed in 0..seeds {
        run(seed);
    }
}
