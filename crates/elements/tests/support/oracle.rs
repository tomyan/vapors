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
    boxes: Vec<(NodeId, Option<BoxRect>)>,
    styles: Vec<Option<ComputedStyle>>,
    items: Vec<String>,
    clips: Vec<Option<Rect>>,
    hits: Vec<(Option<NodeId>, Cursor)>,
    focus: (Option<NodeId>, bool),
    selections: Vec<(NodeId, (usize, usize))>,
}

/// The spacing of the hit grid, CSS px.
const GRID: usize = 10;

/// Updates that a frame may take to settle: one, and one more for a
/// hover the first moved.
const SETTLE: usize = 3;

/// Fails, naming `context`, when the settled incremental result differs
/// from a full recomputation of the same state. The document is left
/// updated from scratch.
pub fn assert_matches_full(doc: &Rc<RefCell<Document>>, viewport: Viewport, context: &str) {
    let kept = snapshot(doc, viewport);
    doc.borrow_mut().forget_kept();
    let fresh = snapshot(doc, viewport);
    assert!(
        kept == fresh,
        "{context}: the incremental update differs from a full one (left: incremental, \
         right: full)\n{}",
        first_difference(&kept, &fresh)
    );
}

/// The settled frame: an update can move content under the pointer,
/// which it notices after painting (it sets the hover flags and asks
/// for another frame), so this updates until no change is pending; at
/// most `SETTLE` times, or the test fails.
pub fn snapshot(doc: &Rc<RefCell<Document>>, viewport: Viewport) -> Snapshot {
    let mut doc = doc.borrow_mut();
    doc.take_changed();
    for _ in 0..SETTLE {
        updated(&mut doc, viewport);
        if !doc.take_changed() {
            return settled(&doc, viewport);
        }
    }
    panic!("the document still changed after {SETTLE} updates");
}

fn updated(doc: &mut Document, viewport: Viewport) {
    doc.update(viewport)
        .unwrap_or_else(|errors| panic!("refused: {errors:#?}"));
}

/// The frame the last update painted, which `snapshot` found settled:
/// another update would only repeat it, and costs seconds at 3000 rows
/// in a debug build (its checks paint and lay out again).
fn settled(doc: &Document, viewport: Viewport) -> Snapshot {
    let nodes = doc.elements();
    let styles = computed_styles(doc, &nodes);
    let focus = (doc.active_element(), doc.focus_visible());
    let selections = selections(doc, &nodes);
    let painted = doc.painted().expect("an update settled");
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

/// The first differing entry of each part, so a failure on thousands of
/// rows names what differs rather than printing both snapshots.
fn first_difference(kept: &Snapshot, fresh: &Snapshot) -> String {
    [
        differ("boxes", &kept.boxes, &fresh.boxes),
        differ("styles", &kept.styles, &fresh.styles),
        differ("items", &kept.items, &fresh.items),
        differ("clips", &kept.clips, &fresh.clips),
        differ("hits", &kept.hits, &fresh.hits),
        differ("focus", &[kept.focus], &[fresh.focus]),
        differ("selections", &kept.selections, &fresh.selections),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("\n")
}

fn differ<T: PartialEq + std::fmt::Debug>(part: &str, kept: &[T], fresh: &[T]) -> Option<String> {
    if kept.len() != fresh.len() {
        return Some(format!(
            "{part}: {} entries, {} full",
            kept.len(),
            fresh.len()
        ));
    }
    let at = kept.iter().zip(fresh).position(|(k, f)| k != f)?;
    Some(format!(
        "{part}[{at}]: {:?}\n  full: {:?}",
        kept[at], fresh[at]
    ))
}
