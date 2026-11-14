//! Debug builds check every update that built by owners against a
//! layout of a fresh box tree with no cache (review-v217 finding 2). A
//! box kept or lent a cache that is out of date, such as a shallow
//! owner given back its old cache after a box below it changed, lays
//! out the same boxes elsewhere or at another size; the walk panics at
//! the first such box, naming its slot and key, at the update where it
//! happens. `rebuild_check.rs` compares the boxes, not their layout.
//!
//! Debug timings (`cargo xtask dev`, touche's debug page) include this
//! full build and layout, O(tree), on every narrowed update. What would
//! retire it: a randomized oracle test that makes several structural
//! changes per update, nested shallow owners in either slot order
//! among them, so the class is caught in tests without the per-update
//! cost.

use taffy::Layout;
use vapors_dom::NodeId;
use vapors_text::FontSystem;

use super::boxes::{Source, build};
use super::{Calcs, Placed, Settling, Slots, TextCache, lay_out};
use crate::update::Viewport;

/// What laying out reads besides the boxes.
pub(crate) struct Inputs<'a> {
    pub(crate) fonts: &'a mut FontSystem,
    pub(crate) calcs: &'a Calcs,
    pub(crate) viewport: Viewport,
}

/// Lays out a fresh build of `root`'s boxes, with a fresh text cache,
/// and walks it from the root beside `placed`; panics at the first box
/// whose size or location differ.
pub(crate) fn check_layout_against_fresh(
    placed: &Placed,
    source: &Source<'_>,
    root: NodeId,
    layout: Inputs<'_>,
) {
    walk(&fresh_layout(source, root, layout), placed);
}

/// `root`'s boxes built and laid out with no cache and no lane, so a
/// lane's layout is compared with Taffy's block layout.
fn fresh_layout(source: &Source<'_>, root: NodeId, layout: Inputs<'_>) -> Placed {
    let fresh = Placed {
        boxes: build(source, root),
        slots: Slots::default(),
        lanes: super::lane::Lanes::default(),
    };
    let viewport = layout.viewport;
    let texts = (layout.fonts, &mut TextCache::default());
    let size = (viewport.width, viewport.height);
    let settling = Settling {
        changed: Vec::new(),
        built: Vec::new(),
        lanes: false,
    };
    let laid = lay_out(fresh, texts, viewport.scale, size, layout.calcs, settling);
    laid.unwrap_or_else(|failed| panic!("a fresh layout failed: {}", failed.0))
        .0
}

/// How far a kept box's location or size may sit from the fresh one's,
/// in CSS px: a lane sums its children's starts and its container's
/// content height in f64 and Taffy in f32, which is not associative,
/// so the last bits can differ (`docs/plans/splice-layout.md`, section
/// 2.1). Every box is compared within it, not only a lane's children:
/// a lane container's height differs by as much, and Taffy carries
/// that into its ancestors' sizes and their later children's
/// locations. A relaxed guard; what would retire it: a Taffy that sums
/// in f64.
///
/// That fixed part covers a short sum. Taffy's positions and sizes are
/// running f32 sums, each addition off by at most half an ulp of its
/// partial sum, and no coordinate takes more additions than the tree
/// has boxes, n: so for sums of non-negative terms (lists of rows) it
/// is off by at most n * EPSILON / 2 * |value|. `tolerance` adds that
/// bound: 4.9 px for a body of 2000 rows of 10.21 px (about 4000 boxes),
/// where Taffy is 0.31 px off the exact 20420. Loose, but a stale row moves a
/// box by a row's height, which is still over it on that page (#348).
const TOLERANCE: f32 = 1.0 / 1024.0;

/// How far `a` and `b` may differ when one was summed in f32 over
/// `terms` terms (above).
#[allow(clippy::cast_precision_loss)] // child counts, far below 2^24
fn tolerance(a: f32, b: f32, terms: usize) -> f32 {
    TOLERANCE + terms as f32 * f32::EPSILON / 2.0 * a.abs().max(b.abs())
}

fn walk(fresh: &Placed, placed: &Placed) {
    match (fresh.boxes.is_empty(), placed.boxes.is_empty()) {
        (true, true) => return,
        (false, false) => {}
        (fresh_empty, _) => panic!(
            "a fresh layout has {} boxes where the kept one has {}: the root's box is \
             out of date (`html {{ display: none }}` builds none); \
             `cargo test -p vapors-elements` reproduces it",
            if fresh_empty { "no" } else { "some" },
            if fresh_empty { "some" } else { "none" },
        ),
    }
    let terms = fresh.boxes.len();
    let apart = |a: f32, b: f32| (a - b).abs() > tolerance(a, b, terms);
    let mut stack = vec![(0, 0)];
    while let Some((index, slot)) = stack.pop() {
        let (new, kept) = (&fresh.boxes[index].layout, &placed.boxes[slot].layout);
        if apart(new.size.width, kept.size.width)
            || apart(new.size.height, kept.size.height)
            || apart(new.location.x, kept.location.x)
            || apart(new.location.y, kept.location.y)
        {
            stale(placed, slot, (new, kept));
        }
        let new_children = fresh.boxes[index].children.iter().copied();
        stack.extend(new_children.zip(placed.boxes[slot].children.iter().copied()));
    }
}

fn stale(placed: &Placed, slot: usize, (new, kept): (&Layout, &Layout)) -> ! {
    panic!(
        "the box in slot {slot}, {:?}, is laid out at {:?} {:?} where a fresh layout puts it \
         at {:?} {:?}: a kept or lent cache is out of date (docs/plans/splice.md (b)); \
         `cargo test -p vapors-elements` reproduces it",
        placed.slots.key(slot),
        kept.location,
        kept.size,
        new.location,
        new.size,
    )
}
