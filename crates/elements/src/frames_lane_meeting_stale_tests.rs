//! The memo refused once the lanes change with no `set_lanes`, on both
//! paths that read it, and framing refusing a gap whole only after
//! rounding (`frames_lane_meeting.rs`, `layout/lane_frame.rs`).

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

/// A framed list of 300 rows, each with a scroll container (a frame
/// under its row's lane frame), a row put in at the top, updated, and
/// every frame meeting the view read (the memo filled).
fn read_list_with_scrollers() -> Document {
    let mut doc = Document::new(FontSystem::bundle_only());
    let css = "body { margin: 0; } .row { height: 20px; } \
        .scroller { height: 20px; overflow: auto; } .tall { height: 40px; }";
    doc.add_stylesheet("p.css", css).expect("parses");
    let list = doc.create_element("div");
    doc.append(doc.body(), list);
    let row_with_scroller = |doc: &mut Document| {
        let row = doc.create_element("div");
        doc.set_attribute(row, "class", "row");
        let scroller = doc.create_element("div");
        doc.set_attribute(scroller, "class", "scroller");
        let tall = doc.create_element("div");
        doc.set_attribute(tall, "class", "tall");
        doc.append(scroller, tall);
        doc.append(row, scroller);
        row
    };
    for _ in 0..300 {
        let row = row_with_scroller(&mut doc);
        doc.append(list, row);
    }
    doc.update(VIEWPORT).expect("updates");
    let row = row_with_scroller(&mut doc);
    let first = doc.children(list)[0];
    doc.insert_before(list, row, Some(first));
    doc.update(VIEWPORT).expect("updates");
    let frames = &doc.painted.as_ref().expect("painted").frames;
    for frame in frames.meeting(Rect::new(0.0, 0.0, 300.0, 200.0)) {
        _ = frames.at(frame);
    }
    doc
}

/// `doc`'s lanes changed with no paint to set them again: the list's
/// lane dropped.
fn lanes_changed_after_set(doc: &Document) {
    let frames = &doc.painted.as_ref().expect("painted").frames;
    let shared = frames.lanes.shared.as_ref().expect("lanes");
    let container = *frames.lanes.bases.keys().next().expect("a framed lane");
    shared.borrow_mut().forget(container);
}

#[test]
#[should_panic(expected = "the lanes changed after the memo was filled")]
fn a_lane_frame_read_from_the_memo_after_the_lanes_changed_is_refused() {
    // Given: the rows' frames memoised
    let doc = read_list_with_scrollers();
    let frames = &doc.painted.as_ref().expect("painted").frames;
    let row = frames.meeting(Rect::new(0.0, 0.0, 300.0, 200.0));
    let row = *row
        .iter()
        .find(|&&frame| frames.lane_container_of(frame).is_some())
        .expect("a row");

    // When: the lanes change, and a row's frame is read
    lanes_changed_after_set(&doc);
    _ = frames.at(row);
}

#[test]
#[should_panic(expected = "the lanes changed after the memo was filled")]
fn a_frame_under_a_lane_row_read_from_the_memo_after_the_lanes_changed_is_refused() {
    // Given: the scroll frames under the rows memoised
    let doc = read_list_with_scrollers();
    let frames = &doc.painted.as_ref().expect("painted").frames;
    let scroller = *frames
        .relative
        .keys()
        .next()
        .expect("a frame under a lane row");
    _ = frames.at(scroller);

    // When: the lanes change, and the scroll frame is read (`memoised`)
    lanes_changed_after_set(&doc);
    _ = frames.at(scroller);
}

#[test]
fn a_gap_that_is_whole_only_after_rounding_is_not_framed() {
    // Given: rows 4 px apart by a gap of 0.8 px, at 1.25: the gap reaches
    // framing as an f32, 0.800000011920929, whose product with 1.25 is not
    // whole; `whole` refuses even the f64 0.8, whose product rounds to 1.0
    // (lane_frame_tests.rs)
    let mut doc = Document::new(FontSystem::bundle_only());
    let css = "body { margin: 0; } .list { display: flex; flex-direction: column; gap: 0.8px; } \
        .row { height: 4px; flex: none; }";
    doc.add_stylesheet("p.css", css).expect("parses");
    let list = doc.create_element("div");
    doc.set_attribute(list, "class", "list");
    doc.append(doc.body(), list);
    for _ in 0..300 {
        let row = doc.create_element("div");
        doc.set_attribute(row, "class", "row");
        doc.append(list, row);
    }
    let viewport = Viewport {
        scale: 1.25,
        ..VIEWPORT
    };
    doc.update(viewport).expect("updates");
    let row = doc.create_element("div");
    doc.set_attribute(row, "class", "row");
    let first = doc.children(list)[0];
    doc.insert_before(list, row, Some(first));

    // When
    doc.update(viewport).expect("updates");

    // Then
    let counts = doc.update_counts();
    assert_eq!(
        counts.lane_unframed,
        Some(crate::LaneDecline::LaneFramePixel),
        "{counts:?}"
    );
}

/// A list of 300 rows styled by `css`, its lane written by a row put in
/// at the top, updated; and the list.
fn written_list(css: &str) -> (Document, vapors_dom::NodeId) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet(
        "p.css",
        &format!("body {{ margin: 0; }} .row {{ height: 20px; }} {css}"),
    )
    .expect("parses");
    let list = doc.create_element("div");
    doc.append(doc.body(), list);
    let row = |doc: &mut Document| {
        let row = doc.create_element("div");
        doc.set_attribute(row, "class", "row");
        row
    };
    for _ in 0..300 {
        let new = row(&mut doc);
        doc.append(list, new);
    }
    doc.update(VIEWPORT).expect("updates");
    let new = row(&mut doc);
    let first = doc.children(list)[0];
    doc.insert_before(list, new, Some(first));
    doc.update(VIEWPORT).expect("updates");
    (doc, list)
}

/// How many times `doc`'s lanes have changed (`Lanes::changes`).
fn changes(doc: &Document) -> u64 {
    let frames = &doc.painted.as_ref().expect("painted").frames;
    frames
        .lanes
        .shared
        .as_ref()
        .expect("lanes")
        .borrow()
        .changes()
}

#[test]
fn a_held_lane_declining_a_later_ask_counts_as_a_change() {
    // Given: a framed list
    let (mut doc, list) = written_list(".out { position: absolute; }");
    let before = changes(&doc);

    // When: a row taken out of flow, so the lane declines its next ask
    // and is dropped
    let row = doc.children(list)[5];
    doc.set_attribute(row, "class", "row out");
    doc.update(VIEWPORT).expect("updates");

    // Then
    assert!(
        doc.update_counts().lanes_declined > 0,
        "{:?}",
        doc.update_counts()
    );
    assert!(changes(&doc) > before, "{before} -> {}", changes(&doc));
}

#[test]
fn a_lane_made_for_an_ask_and_declined_counts_no_change() {
    // Given / When: a list whose first row is out of flow from the start,
    // so every ask declines and no lane is ever held
    let mut doc = Document::new(FontSystem::bundle_only());
    let css = "body { margin: 0; } .row { height: 20px; } .out { position: absolute; }";
    doc.add_stylesheet("p.css", css).expect("parses");
    let list = doc.create_element("div");
    doc.append(doc.body(), list);
    for at in 0..300 {
        let row = doc.create_element("div");
        doc.set_attribute(row, "class", if at == 0 { "row out" } else { "row" });
        doc.append(list, row);
    }
    doc.update(VIEWPORT).expect("updates");

    // Then
    assert!(
        doc.update_counts().lanes_declined > 0,
        "{:?}",
        doc.update_counts()
    );
    assert_eq!(changes(&doc), 0);
}
