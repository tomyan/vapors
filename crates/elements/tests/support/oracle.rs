//! The correctness oracle for incremental updates (IU1): a document's
//! settled result after its edits, compared with the result of the same
//! document once it forgets everything kept between updates
//! (`Document::forget_kept`) and updates from scratch. Compared: each
//! element's border box and computed style in tree order; the display
//! list in paint order, each text run as its text, origin, layout size
//! and every glyph run (size, colour, glyph ids and positions), each
//! icon with its paths, with
//! each item's clip; the element hit and the cursor at every `GRID` CSS px; focus;
//! and each text field's selection.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_css::cascade::ComputedStyle;
use vapors_dom::NodeId;
use vapors_elements::{BoxRect, Cursor, Document, Painted, Viewport};
use vapors_scene::{Item, Rect};
use vapors_text::Layout;

/// What the oracle compares.
#[derive(Debug, PartialEq)]
pub struct Snapshot {
    pub(super) boxes: Vec<(NodeId, Option<BoxRect>)>,
    pub(super) styles: Vec<Option<ComputedStyle>>,
    pub(super) items: Vec<String>,
    pub(super) clips: Vec<Option<Rect>>,
    pub(super) hits: Vec<(Option<NodeId>, Cursor)>,
    pub(super) focus: (Option<NodeId>, bool),
    pub(super) selections: Vec<(NodeId, (usize, usize))>,
}

/// The spacing of the hit grid, CSS px.
const GRID: usize = 10;

/// Updates that a frame may take to settle: one, and one more for a
/// hover the first moved.
const SETTLE: usize = 3;

/// Whether a snapshot runs one more update after finding the frame
/// settled, and compares that update's frame.
#[derive(Clone, Copy)]
enum Repeat {
    /// The repeat runs, with the debug checks it brings.
    Run,
    /// The settle update's frame is compared, as the repeat would only
    /// reproduce it; for tests where each update costs seconds.
    Skip,
}

/// Fails, naming `context`, when the settled incremental result differs
/// from a full recomputation of the same state. The document is left
/// updated from scratch.
pub fn assert_matches_full(doc: &Rc<RefCell<Document>>, viewport: Viewport, context: &str) {
    assert_matches_full_with(doc, viewport, context, Repeat::Run);
}

/// [`assert_matches_full`] without the repeat update of each snapshot:
/// the 3000-row `kept_shift` tests, where each update takes seconds in a
/// debug build. Retire this when those tests' updates take well under a
/// second (the test profile's `opt-level`, measured in vapors #233).
pub fn assert_matches_full_without_repeat(
    doc: &Rc<RefCell<Document>>,
    viewport: Viewport,
    context: &str,
) {
    assert_matches_full_with(doc, viewport, context, Repeat::Skip);
}

fn assert_matches_full_with(
    doc: &Rc<RefCell<Document>>,
    viewport: Viewport,
    context: &str,
    repeat: Repeat,
) {
    let kept = snapshot_with(doc, viewport, repeat);
    doc.borrow_mut().forget_kept();
    let fresh = snapshot_with(doc, viewport, repeat);
    assert!(
        kept == fresh,
        "{context}: the incremental update differs from a full one (left: incremental, \
         right: full)\n{}",
        crate::support::oracle_difference::first_difference(&kept, &fresh)
    );
}

/// The settled frame: an update can move content under the pointer,
/// which it notices after painting (it sets the hover flags and asks
/// for another frame), so this updates until no change is pending; at
/// most `SETTLE` times, or the test fails.
pub fn snapshot(doc: &Rc<RefCell<Document>>, viewport: Viewport) -> Snapshot {
    snapshot_with(doc, viewport, Repeat::Run)
}

fn snapshot_with(doc: &Rc<RefCell<Document>>, viewport: Viewport, repeat: Repeat) -> Snapshot {
    let mut doc = doc.borrow_mut();
    doc.take_changed();
    for _ in 0..SETTLE {
        updated(&mut doc, viewport);
        if !doc.take_changed() {
            return settled(&mut doc, viewport, repeat);
        }
    }
    panic!("the document still changed after {SETTLE} updates");
}

fn updated(doc: &mut Document, viewport: Viewport) -> &Painted {
    doc.update(viewport)
        .unwrap_or_else(|errors| panic!("refused: {errors:#?}"))
}

fn settled(doc: &mut Document, viewport: Viewport, repeat: Repeat) -> Snapshot {
    let nodes = doc.elements();
    let styles = computed_styles(doc, &nodes);
    let focus = (doc.active_element(), doc.focus_visible());
    let selections = selections(doc, &nodes);
    let painted = match repeat {
        Repeat::Run => updated(doc, viewport),
        // `snapshot_with` calls this after a successful update
        Repeat::Skip => doc.painted().expect("the settle update succeeded"),
    };
    Snapshot {
        boxes: border_boxes(painted, &nodes),
        styles,
        items: items(painted),
        clips: painted.scene().to_scene().clips().to_vec(),
        hits: hits(painted, viewport),
        focus,
        selections,
    }
}

fn computed_styles(doc: &Document, nodes: &[NodeId]) -> Vec<Option<ComputedStyle>> {
    nodes
        .iter()
        .map(|n| doc.computed_style(*n).cloned())
        .collect()
}

fn selections(doc: &Document, nodes: &[NodeId]) -> Vec<(NodeId, (usize, usize))> {
    nodes
        .iter()
        .filter_map(|n| doc.selection(*n).map(|s| (*n, s)))
        .collect()
}

fn border_boxes(painted: &Painted, nodes: &[NodeId]) -> Vec<(NodeId, Option<BoxRect>)> {
    nodes.iter().map(|n| (*n, painted.border_box(*n))).collect()
}

fn hits(painted: &Painted, viewport: Viewport) -> Vec<(Option<NodeId>, Cursor)> {
    grid(viewport)
        .map(|point| (painted.hit(point), painted.cursor(point)))
        .collect()
}

fn items(painted: &Painted) -> Vec<String> {
    let text = |item: &Item| match item {
        Item::Text { text, origin, .. } => {
            format!("text {} at {origin:?}", layout_detail(painted.text(*text)))
        }
        // the icon's paths, so a stale svg differs
        Item::Icon { icon, .. } => format!("{item:?} {:?}", painted.icon(*icon)),
        other => format!("{other:?}"),
    };
    painted
        .scene()
        .to_scene()
        .items()
        .iter()
        .map(text)
        .collect()
}

/// A text layout as the string, its size and every run, so a stale
/// colour, size or glyph position differs.
fn layout_detail(layout: &Layout) -> String {
    format!(
        "{:?} {}x{} runs {:?}",
        layout.text(),
        layout.width(),
        layout.height(),
        layout
            .runs()
            .iter()
            .map(|run| (run.size, run.color, &run.glyphs))
            .collect::<Vec<_>>()
    )
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
