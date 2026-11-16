use vapors_dom::{Dom, NodeId};
use vapors_text::FontSystem;

use crate::document::Document;
use crate::update::Viewport;

const VIEWPORT: Viewport = Viewport {
    width: 300.0,
    height: 200.0,
    scale: 1.0,
};

/// `count` list items of class `class` under `parent`.
fn items(doc: &mut Document, parent: NodeId, count: usize, class: &str) -> Vec<NodeId> {
    (0..count)
        .map(|_| {
            let item = doc.create_element("div");
            doc.set_attribute(item, "class", class);
            doc.set_attribute(item, "role", "listitem");
            doc.append(parent, item);
            item
        })
        .collect()
}

/// `doc` updated and its a11y tree taken, as a frame does.
fn updated_and_taken(doc: &mut Document) {
    doc.update(VIEWPORT).expect("updates");
    doc.take_ax_update().expect("taken");
}

/// The lane frames folded by the take after a row is put in at the top
/// of a framed list of `rows` rows.
fn folds_after_an_insert(rows: usize) -> usize {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet("p.css", "body { margin: 0; } .row { height: 20px; }")
        .expect("parses");
    let body = doc.body();
    let list = doc.create_element("div");
    doc.append(body, list);
    let first = items(&mut doc, list, rows, "row")[0];
    updated_and_taken(&mut doc);
    let row = doc.create_element("div");
    doc.set_attribute(row, "class", "row");
    doc.set_attribute(row, "role", "listitem");
    doc.insert_before(list, row, Some(first));
    updated_and_taken(&mut doc);
    doc.painted.as_ref().expect("painted").frames.lane_folds()
}

#[test]
fn the_take_after_an_insert_folds_no_started_row() {
    // When: a row put in at the top of 100 rows and of 300
    let (small, large) = (folds_after_an_insert(100), folds_after_an_insert(300));

    // Then: every later row started elsewhere and was primed, so the
    // folds do not grow with the rows
    assert_eq!(small, large, "the take folds each started row");
}

#[test]
fn a_framed_lane_in_a_framed_row_is_primed_after_an_insert() {
    // Given: 40 framed rows, each a framed lane of 40 rows
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet("p.css", "body { margin: 0; } .inner { height: 2px; }")
        .expect("parses");
    let body = doc.body();
    let list = doc.create_element("div");
    doc.append(body, list);
    let outer = items(&mut doc, list, 40, "outer");
    let inner: Vec<Vec<NodeId>> = outer
        .iter()
        .map(|&row| items(&mut doc, row, 40, "inner"))
        .collect();
    updated_and_taken(&mut doc);

    // When: a row put in at the top of the third row's lane
    let row = doc.create_element("div");
    doc.set_attribute(row, "class", "inner");
    doc.insert_before(outer[2], row, Some(inner[2][0]));

    // Then: the take primes the inner lane's rows, whose outer frame is
    // a lane frame, without a borrow panic
    updated_and_taken(&mut doc);
}
