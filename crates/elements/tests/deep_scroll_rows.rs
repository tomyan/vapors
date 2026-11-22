//! Rows tens of millions of CSS px down a scroll frame keep the
//! fractions of a pixel in their boxes (virtual list V3: a million 52 px
//! rows is 52,000,000 px, where an f32 steps by 4 px). A lane whose
//! children start `FRAMED_DEPTH` down frames them however few
//! (`layout/lane.rs`), so each row's boxes are placed in its own frame
//! near zero and shifted in f64; the debug placement check composes a
//! framed child's fresh rect from its place in its frame, as placement
//! does (`layout::placed`'s `framed`).

use crate::support::{document, element};
use vapors_dom::NodeId;
use vapors_elements::{Document, Viewport};

const VIEW: Viewport = Viewport {
    width: 400.0,
    height: 400.0,
    scale: 1.0,
};

/// The pills' tops, CSS px on the page, when the last row's bottom is at
/// the bottom of the 156 px list and the list is then wheeled one row
/// up: each pill is centred in its 52 px row, 15.5 px down it.
const EXACT: [f32; 3] = [171.5, 119.5, 67.5];

struct Page {
    doc: Document,
    list: NodeId,
    pills: Vec<NodeId>,
}

/// A 156 px list holding a spacer `spacer` px tall, then `rows` rows of
/// 52 px, each centring a 21 px pill.
fn page(rows: usize, spacer: f32) -> Page {
    let mut doc = document();
    let body = doc.body();
    let list = element(&mut doc, body, "div", "list");
    let content = element(&mut doc, list, "div", "content");
    element(&mut doc, content, "div", "spacer");
    let pills = (0..rows)
        .map(|_| {
            let row = element(&mut doc, content, "div", "row");
            element(&mut doc, row, "div", "pill")
        })
        .collect();
    let css = format!(
        "body {{ margin: 0; }} .list {{ width: 300px; height: 156px; overflow-y: auto; }} \
         .spacer {{ height: {spacer}px; }} \
         .row {{ height: 52px; display: flex; align-items: center; }} \
         .pill {{ width: 20px; height: 21px; }}"
    );
    doc.add_stylesheet("app.css", &css).expect("parses");
    Page { doc, list, pills }
}

/// The last three pills' tops after a scroll to the end and a wheel one
/// row back, each followed by an update (the debug build checks every
/// kept placement against a fresh one).
fn last_pills_after_scrolling(mut page: Page) -> Vec<f32> {
    page.doc.update(VIEW).expect("updates");
    page.doc.scroll_to(page.list, [0.0, f32::MAX]);
    page.doc.update(VIEW).expect("updates");
    page.doc.scroll_by([10.0, 10.0], [0.0, -52.0]);
    page.doc.update(VIEW).expect("updates");
    let painted = page.doc.painted().expect("painted");
    let top = |pill: &NodeId| painted.border_box(*pill).expect("a box").y;
    page.pills.iter().rev().take(3).map(top).collect()
}

#[test]
fn a_few_rows_52_million_px_down_keep_their_half_pixel_boxes() {
    // Given: four rows below a spacer of 999,996 rows
    let page = page(4, 52.0 * 999_996.0);

    // Then
    assert_eq!(last_pills_after_scrolling(page), EXACT);
}

#[test]
fn many_rows_52_million_px_down_keep_their_half_pixel_boxes() {
    // Given: forty rows, a lane framed by its count
    let page = page(40, 52.0 * 999_960.0);

    // Then
    assert_eq!(last_pills_after_scrolling(page), EXACT);
}

#[test]
fn a_few_rows_near_the_top_keep_their_half_pixel_boxes() {
    // Given: rows above FRAMED_DEPTH, where an f32 holds the half pixels
    let page = page(4, 52.0 * 96.0);

    // Then
    assert_eq!(last_pills_after_scrolling(page), EXACT);
}
