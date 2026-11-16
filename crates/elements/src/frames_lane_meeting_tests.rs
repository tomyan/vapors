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
    inserted_at_the_top_of("20px", VIEWPORT)
}

/// A framed list of 300 rows `height` tall with a row put in at the top,
/// updated at `viewport`.
fn inserted_at_the_top_of(height: &str, viewport: Viewport) -> Document {
    let mut doc = Document::new(FontSystem::bundle_only());
    let css = format!("body {{ margin: 0; }} .row {{ height: {height}; background-color: #f00; }}");
    doc.add_stylesheet("p.css", &css).expect("parses");
    let list = doc.create_element("div");
    doc.append(doc.body(), list);
    for _ in 0..300 {
        let row = doc.create_element("div");
        doc.set_attribute(row, "class", "row");
        doc.append(list, row);
    }
    doc.update(viewport).expect("updates");
    let row = doc.create_element("div");
    doc.set_attribute(row, "class", "row");
    let first = doc.children(list)[0];
    doc.insert_before(list, row, Some(first));
    doc.update(viewport).expect("updates");
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

#[test]
fn the_search_and_the_fold_give_a_row_one_start_at_fractional_scales() {
    // Given: 300 rows (a tree of depth 3: the search adds starts
    // top-down, the fold bottom-up), framed, at the Pixel's scale, at
    // 1.25 and in half pixels at 2: framing takes only extents a whole
    // number of device pixels, exactly (`lane_frame.rs` `whole`), so every
    // start is a sum of exact binary fractions and either order gives it
    for (height, scale) in [("4px", 1.25), ("4px", 2.75), ("0.5px", 2.0)] {
        let viewport = Viewport { scale, ..VIEWPORT };
        let doc = inserted_at_the_top_of(height, viewport);
        let counts = doc.update_counts();
        assert!(counts.lanes_framed > 0, "{height} at {scale}: {counts:?}");

        // When: the rows meeting the view found, each memoised from its
        // start (the debug check holds each to the lane's fold)
        let frames = &doc.painted.as_ref().expect("painted").frames;
        let found = frames.meeting(Rect::new(0.0, 0.0, 300.0 * scale, 200.0 * scale));

        // Then
        assert!(found.len() > 10, "{height} at {scale}: {found:?}");
    }
}

#[test]
fn rows_whose_height_is_not_exact_in_device_pixels_are_not_framed() {
    // Given: 2.4 CSS px is 3.0000001 device px at 1.25 in binary
    let viewport = Viewport {
        scale: 1.25,
        ..VIEWPORT
    };

    // When
    let doc = inserted_at_the_top_of("2.4px", viewport);

    // Then: declined, so no start of such a row is summed in two orders
    let counts = doc.update_counts();
    assert_eq!(
        counts.lane_unframed,
        Some(crate::LaneDecline::LaneFramePixel)
    );
}

/// The lane frames `doc`'s frames folded while the frames meeting `view`
/// were found and each read.
fn folds_reading_meeting(doc: &Document, view: Rect) -> (usize, usize) {
    let frames = &doc.painted.as_ref().expect("painted").frames;
    let before = frames.lane_folds();
    let found = frames.meeting(view);
    for &frame in &found {
        _ = frames.at(frame);
    }
    (found.len(), frames.lane_folds() - before)
}

#[test]
fn the_rows_of_a_lane_scrolled_partway_are_read_without_folding_the_lane() {
    // Given: the body scrolls a framed list of 300 rows, half a row past
    // its fiftieth
    let mut doc = Document::new(FontSystem::bundle_only());
    let css = "html { overflow: hidden; } body { margin: 0; overflow: auto; height: 200px; } \
        .row { height: 20px; background-color: #f00; }";
    doc.add_stylesheet("p.css", css).expect("parses");
    let list = doc.create_element("div");
    doc.append(doc.body(), list);
    for _ in 0..300 {
        let row = doc.create_element("div");
        doc.set_attribute(row, "class", "row");
        doc.append(list, row);
    }
    doc.update(VIEWPORT).expect("updates");
    let first = doc.children(list)[0];
    let row = doc.create_element("div");
    doc.set_attribute(row, "class", "row");
    doc.insert_before(list, row, Some(first));
    doc.update(VIEWPORT).expect("updates");
    assert!(
        doc.scroll_node_by(doc.body(), [0.0, 1010.0]),
        "the body scrolls"
    );
    doc.update(VIEWPORT).expect("updates");

    // When
    let (found, folds) = folds_reading_meeting(&doc, Rect::new(0.0, 0.0, 300.0, 200.0));

    // Then
    assert!(found > 10, "{found}");
    assert_eq!(folds, 0);
}

#[test]
fn the_rows_of_a_lane_in_a_framed_row_are_read_without_folding_either_lane() {
    // Given: a framed list of 40 rows, each a framed list of 40 cells
    let mut doc = Document::new(FontSystem::bundle_only());
    let css = "body { margin: 0; } .cell { height: 2px; background-color: #f00; }";
    doc.add_stylesheet("p.css", css).expect("parses");
    let outer = doc.create_element("div");
    doc.append(doc.body(), outer);
    for _ in 0..40 {
        let row = doc.create_element("div");
        doc.append(outer, row);
        for _ in 0..40 {
            let cell = doc.create_element("div");
            doc.set_attribute(cell, "class", "cell");
            doc.append(row, cell);
        }
    }
    doc.update(VIEWPORT).expect("updates");
    let rows = doc.children(outer);
    for row in [rows[0], rows[1]] {
        let cell = doc.create_element("div");
        doc.set_attribute(cell, "class", "cell");
        let first = doc.children(row)[0];
        doc.insert_before(row, cell, Some(first));
    }
    doc.update(VIEWPORT).expect("updates");
    assert!(
        doc.update_counts().lanes_framed > 1,
        "{:?}",
        doc.update_counts()
    );

    // When
    let (found, folds) = folds_reading_meeting(&doc, Rect::new(0.0, 0.0, 300.0, 200.0));

    // Then: the outer row is found (and memoised) before the lane in it
    // is searched, so neither lane is folded
    assert!(found > 10, "{found}");
    assert_eq!(folds, 0);
}

#[test]
fn frames_made_and_never_given_lanes_have_epochs_of_their_own() {
    // Given / When
    let (a, b) = (
        crate::frames::Frames::page(1.0),
        crate::frames::Frames::page(1.0),
    );

    // Then: a painter keeping places by epoch tells them apart
    assert_ne!(a.epoch(), b.epoch());
}
