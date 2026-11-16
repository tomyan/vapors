//! The cases #415's review found the patch at a lane's ends got wrong,
//! each with the debug check (`assert_candidates`, after every draw)
//! active: a lane with ink, a relabel onto another shown step's label, a
//! frame inside a row, and a lane whose rows come into view.

use vapors_dom::{Dom, NodeId};
use vapors_scene::Rect;

use crate::document::Document;
use crate::update::Viewport;

/// 300 x 210 device pixels.
const VIEWPORT: Viewport = Viewport {
    width: 300.0,
    height: 210.0,
    scale: 1.0,
};

/// A framed list of 200 rows under a body that scrolls, styled by
/// `css`, each row filled by `fill`, scrolled by `by`, and drawn.
fn list(css: &str, fill: impl Fn(&mut Document, NodeId), by: f32) -> (Document, NodeId) {
    let mut doc = Document::new(vapors_text::FontSystem::bundle_only());
    let page = "html { overflow: hidden; } body { margin: 0; overflow: auto; height: 210px; } \
        .row { height: 20px; background-color: #f00; }";
    doc.add_stylesheet("p.css", &format!("{page} {css}"))
        .expect("parses");
    let list = doc.create_element("div");
    doc.set_attribute(list, "class", "list");
    doc.append(doc.body(), list);
    for _ in 0..200 {
        let row = row(&mut doc, list, None);
        fill(&mut doc, row);
    }
    doc.update(VIEWPORT).expect("updates");
    row(&mut doc, list, None);
    doc.update(VIEWPORT).expect("updates");
    assert!(
        doc.scroll_node_by(doc.body(), [0.0, by]),
        "the body scrolls"
    );
    drawn(&mut doc);
    (doc, list)
}

/// A row put in `list` before `before` (at the end for `None`).
fn row(doc: &mut Document, list: NodeId, before: Option<NodeId>) -> NodeId {
    let row = doc.create_element("div");
    doc.set_attribute(row, "class", "row");
    doc.insert_before(list, row, before);
    row
}

/// `doc` updated and its batches drawn (the debug check runs).
fn drawn(doc: &mut Document) {
    let painted = doc.update(VIEWPORT).expect("updates");
    _ = painted.kept_batches(Rect::new(0.0, 0.0, 300.0, 210.0));
}

/// A row put in at the top of `list`, and the page drawn.
fn put_in_at_the_top(doc: &mut Document, list: NodeId) {
    let first = doc.children(list)[0];
    row(doc, list, Some(first));
    drawn(doc);
}

#[test]
fn a_row_put_in_above_a_lane_with_ink_draws_the_row_the_view_cuts() {
    // Given: rows 20 px tall, the first with a shadow 40 px below it, so
    // the lane's ink reaches two rows past a row's box; scrolled so a row
    // lies just above the view
    let shadow = |doc: &mut Document, row: NodeId| {
        if doc.children(doc.parent(row).expect("a list")).first() == Some(&row) {
            doc.set_attribute(row, "class", "row inked");
        }
    };
    let css = ".inked { box-shadow: 0 40px 0 0 #000; }";
    let (mut doc, list) = list(css, shadow, 1000.0);

    // When / Then: the rows move down one; the row now cut by the view's
    // top is drawn (the debug check)
    put_in_at_the_top(&mut doc, list);
}

#[test]
fn rows_with_a_scroll_box_each_come_into_view_with_its_content() {
    // Given: each row holds a box that scrolls its taller content
    let scroller = |doc: &mut Document, row: NodeId| {
        let scroller = doc.create_element("div");
        doc.set_attribute(scroller, "class", "scroller");
        let content = doc.create_element("div");
        doc.set_attribute(content, "class", "content");
        doc.append(scroller, content);
        doc.append(row, scroller);
    };
    let css = ".scroller { height: 20px; overflow: auto; } \
        .content { height: 40px; background-color: #00f; }";
    let (mut doc, list) = list(css, scroller, 1010.0);

    // When / Then: a row comes into view at the top, one leaves at the
    // bottom, each with its scroll box's content (the debug check)
    put_in_at_the_top(&mut doc, list);
}

#[test]
fn a_lane_whose_rows_come_into_view_from_above_it_is_searched() {
    // Given: the list's rows all above the view; its padding below them
    // is what the view shows
    let css = ".list { padding-bottom: 400px; }";
    let (mut doc, list) = list(css, |_, _| (), 4030.0);

    // When / Then: a row put in at the top moves the last row into view
    // (the debug check)
    put_in_at_the_top(&mut doc, list);
}

#[test]
fn a_relabel_onto_another_shown_steps_old_label_keeps_both_steps() {
    // Given: the steps shown in the list
    let (doc, _) = list("", |_, _| (), 1010.0);
    let painted = doc.painted.as_ref().expect("painted");
    let view = Rect::new(0.0, 0.0, 300.0, 210.0);
    let mut kept = crate::kept_batches::KeptBatches::of(view, painted.batch_sources());
    let (first, second, third) = (kept.shown[0], kept.shown[1], kept.shown[2]);

    // When: a splice moves the first onto the second's old label, and the
    // second into the gap before the third
    let between = second.label + (third.label - second.label) / 2;
    let edits = crate::kept_order_splice::SpliceEdits {
        relabelled: vec![
            (first.step, first.label, second.label),
            (second.step, second.label, between),
        ],
        ..Default::default()
    };
    kept.spliced(&edits);

    // Then: both steps are shown, at their new labels
    let at = |step| {
        kept.shown
            .iter()
            .find(|shown| shown.step == step)
            .map(|shown| shown.label)
    };
    assert_eq!(
        (at(first.step), at(second.step)),
        (Some(second.label), Some(between))
    );
}

#[test]
fn an_end_row_taken_out_as_a_row_is_put_in_sorts_the_steps_again() {
    // Given
    let (mut doc, list) = list("", |_, _| (), 1010.0);

    // When: in one update, the row the view's top cuts taken out and a
    // row put in at the top, which may take its slot
    let top = doc.children(list)[50];
    doc.remove(top);
    let first = doc.children(list)[0];
    row(&mut doc, list, Some(first));
    doc.update(VIEWPORT).expect("updates");

    // Then: counted as sorted again, and drawn as a full search finds
    // (the debug check)
    assert_eq!(doc.update_counts().batch_steps_sorted_again, 1);
    drawn(&mut doc);
}
