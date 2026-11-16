use vapors_dom::{Dom, NodeId};
use vapors_text::FontSystem;

use crate::document::Document;
use crate::update::Viewport;

const VIEWPORT: Viewport = Viewport {
    width: 300.0,
    height: 200.0,
    scale: 1.0,
};

const CSS: &str = "body { margin: 0; } .row { height: 20px; } \
    .scroll { overflow: auto; height: 20px; } .tall { height: 60px; } \
    .inner { overflow: auto; height: 10px; }";

/// An element of class `class` appended to `parent`.
fn child(doc: &mut Document, parent: NodeId, class: &str) -> NodeId {
    let node = doc.create_element("div");
    doc.set_attribute(node, "class", class);
    doc.append(parent, node);
    node
}

/// A framed list of `count` rows, each a scroller over a tall child
/// holding a scroller of its own; the list and each row's two scrollers.
fn rows_with_nested_scrollers(doc: &mut Document, count: usize) -> (NodeId, Vec<[NodeId; 2]>) {
    doc.add_stylesheet("p.css", CSS).expect("parses");
    let body = doc.body();
    let list = child(doc, body, "list");
    let rows = (0..count)
        .map(|_| {
            let row = child(doc, list, "row");
            let outer = child(doc, row, "scroll");
            let tall = child(doc, outer, "tall");
            let inner = child(doc, tall, "inner");
            child(doc, inner, "tall");
            [outer, inner]
        })
        .collect();
    (list, rows)
}

/// Reads every scroll frame of `doc`'s last paint, filling the memo.
fn read_every_scroll_frame(doc: &Document) {
    let frames = &doc.painted.as_ref().expect("painted").frames;
    for &slot in frames.relative.keys() {
        frames.at(slot);
    }
}

#[test]
fn every_frame_read_after_a_scroll_in_a_framed_row_is_the_one_made_afresh() {
    // Given: 50 framed rows, a row put in at the top, so every lane
    // frame below it moved
    let mut doc = Document::new(FontSystem::bundle_only());
    let (list, rows) = rows_with_nested_scrollers(&mut doc, 50);
    doc.update(VIEWPORT).expect("updates");
    let first = doc.children(list)[0];
    let row = doc.create_element("div");
    doc.set_attribute(row, "class", "row");
    doc.insert_before(list, row, Some(first));
    doc.update(VIEWPORT).expect("updates");
    // and every scroll frame read, as a draw reads those it shows
    read_every_scroll_frame(&doc);

    // When: a row's scroller scrolled, and the one nested in it
    assert!(doc.scroll_node_by(rows[3][0], [0.0, 5.0]));
    assert!(doc.scroll_node_by(rows[3][1], [0.0, 7.0]));
    doc.update(VIEWPORT).expect("updates");

    // Then: every scroll frame is kept by its own part, and each, read
    // through the memo, is the frame composed afresh
    let frames = &doc.painted.as_ref().expect("painted").frames;
    assert_eq!(frames.relative.len(), 100, "two scroll frames a row");
    let mut slots: Vec<usize> = frames.relative.keys().copied().collect();
    slots.sort_unstable();
    for slot in slots {
        let afresh = frames.resolved_afresh(slot);
        assert_eq!(frames.at(slot), afresh, "frame {slot}");
    }
}
