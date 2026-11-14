use vapors_dom::{Dom, NodeId};
use vapors_text::FontSystem;

use crate::document::Document;
use crate::update::Viewport;

const VIEWPORT: Viewport = Viewport {
    width: 200.0,
    height: 200.0,
    scale: 1.0,
};

/// A framed list of `rows` rows 20px tall under body, laid out; the
/// rows.
fn list(rows: usize) -> (Document, Vec<NodeId>) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet("app.css", "body { margin: 0; } .row { height: 20px; }")
        .expect("parses");
    let body = doc.body();
    let list = doc.create_element("div");
    doc.append(body, list);
    let rows = (0..rows)
        .map(|_| {
            let row = doc.create_element("div");
            doc.set_attribute(row, "class", "row");
            doc.append(list, row);
            row
        })
        .collect();
    (doc, rows)
}

/// The element hit at `point` in `rows` rows, and how many hits that
/// tested.
fn hit_in(rows: usize, point: [f32; 2]) -> (Option<NodeId>, usize, Vec<NodeId>) {
    let (mut doc, nodes) = list(rows);
    let painted = doc.update(VIEWPORT).expect("updates");
    let (hit, tested) = painted.kept_hits().hit_counted(point, &painted.frames);
    (hit, tested, nodes)
}

#[test]
fn a_hit_in_a_framed_list_finds_its_row() {
    // When
    let (hit, _, rows) = hit_in(100, [10.0, 105.0]);

    // Then: the sixth row, 100 to 120px down
    assert_eq!(hit, Some(rows[5]));
}

#[test]
fn a_hit_in_a_framed_list_tests_the_rows_at_its_point_alone() {
    // When: a hit test in 100 rows and in 400
    let (_, small, _) = hit_in(100, [10.0, 105.0]);
    let (_, large, _) = hit_in(400, [10.0, 105.0]);

    // Then: as many hits tested at either size
    assert_eq!(small, large, "a hit test reads every row's hits");
}
