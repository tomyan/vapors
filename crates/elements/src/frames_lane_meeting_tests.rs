use vapors_dom::Dom;
use vapors_scene::Rect;
use vapors_text::FontSystem;

use crate::document::Document;
use crate::update::Viewport;

const VIEWPORT: Viewport = Viewport {
    width: 300.0,
    height: 200.0,
    scale: 1.0,
};

/// A framed list of 200 rows with a row put in at the top, updated.
fn inserted_at_the_top() -> Document {
    let mut doc = Document::new(FontSystem::bundle_only());
    let css = "body { margin: 0; } .row { height: 20px; background-color: #f00; }";
    doc.add_stylesheet("p.css", css).expect("parses");
    let list = doc.create_element("div");
    doc.append(doc.body(), list);
    for _ in 0..200 {
        let row = doc.create_element("div");
        doc.set_attribute(row, "class", "row");
        doc.append(list, row);
    }
    doc.update(VIEWPORT).expect("updates");
    let row = doc.create_element("div");
    doc.set_attribute(row, "class", "row");
    let first = doc.children(list)[0];
    doc.insert_before(list, row, Some(first));
    doc.update(VIEWPORT).expect("updates");
    doc
}

#[test]
fn the_rows_a_lane_search_finds_are_read_without_folding_the_lane() {
    // Given
    let doc = inserted_at_the_top();
    let frames = &doc.painted.as_ref().expect("painted").frames;
    let before = frames.lane_folds();

    // When: the frames meeting the view are found, and each is read
    let found = frames.meeting(Rect::new(0.0, 0.0, 300.0, 200.0));
    for &frame in &found {
        _ = frames.at(frame);
    }

    // Then: the rows in view were found, each with its start, so no read
    // of one folded the lane
    assert!(found.len() > 10, "{found:?}");
    assert_eq!(frames.lane_folds() - before, 0);
}
