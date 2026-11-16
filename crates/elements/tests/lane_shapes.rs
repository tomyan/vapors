//! Shapes in the rows of a framed lane (`docs/plans/splice-layout.md`
//! section 11.2, 4f; option (b) of the fold): a step that paints a shape
//! or a mesh is batched in its row's own frame, as the shape and mesh
//! pipelines take no table of row offsets. As batches keep paint order,
//! the lane's batch is cut at each such row: here each row's background
//! is a lane batch of one row. Kept batches match a fresh batching after
//! a scroll and an insert (the debug check in `kept_batches`).

use crate::support;

use std::cell::RefCell;
use std::rc::Rc;

use support::lane_page::assert_drawn_afresh;
use support::svg_page::{SVG, with_attributes};
use support::{document, element, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};
use vapors_scene::Run;

const RECT: &[(&str, &str)] = &[
    ("x", "2"),
    ("y", "2"),
    ("width", "12"),
    ("height", "12"),
    ("fill", "blue"),
];
const PATH: &[(&str, &str)] = &[("d", "M2 2 L14 2 L8 14 Z"), ("fill", "green")];

/// A scrolling body holding a list of `count` 20 px rows, each with a
/// background and an svg holding a rect (closed form) and a path (mesh);
/// the document and the list.
fn rows_with_shapes(count: usize, view: Viewport) -> (Rc<RefCell<Document>>, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let list = element(&mut doc, body, "div", "list");
    for _ in 0..count {
        add_row(&mut doc, list, None);
    }
    let css = "html { overflow: hidden; } body { margin: 0; overflow: auto; height: 300px; } \
               .row { height: 20px; background-color: #c33; } .icon { display: block; }";
    doc.add_stylesheet("page.css", css).expect("parses");
    doc.update(view).expect("updates");
    (Rc::new(RefCell::new(doc)), list)
}

/// A row with a background and the two shapes, put in `list` before
/// `before` (at the end when `None`).
fn add_row(doc: &mut Document, list: NodeId, before: Option<NodeId>) {
    let row = doc.create_element("div");
    doc.set_attribute(row, "class", "row");
    doc.insert_before(list, row, before);
    let svg = element(doc, row, "svg", "icon");
    with_attributes(doc, svg, SVG);
    for (tag, attributes) in [("rect", RECT), ("path", PATH)] {
        let shape = element(doc, svg, tag, "");
        with_attributes(doc, shape, attributes);
    }
}

/// For each batch drawn at `view` with items: whether its run is of
/// shapes or meshes, and how many items name a lane row.
fn batches(doc: &Rc<RefCell<Document>>, view: Viewport) -> Vec<(bool, usize)> {
    let mut doc = doc.borrow_mut();
    let painted = doc.update(view).expect("updates");
    let rect = vapors_scene::Rect::new(0.0, 0.0, view.width * view.scale, view.height * view.scale);
    let kept = painted.kept_batches(rect).clone();
    let runs = kept.batched.batches.iter().map(|batch| &batch.run);
    let shaped = runs.map(|run| matches!(run, Run::Shapes(_) | Run::Meshes(_)));
    let rows = kept.rows.iter().map(|rows| rows.items().len());
    shaped.zip(rows).collect()
}

#[test]
fn shapes_in_lane_rows_are_batched_in_their_rows_frames() {
    // Given: 100 rows, 15 in a 300 px view, each with a shape and a mesh
    let view = viewport(400.0, 300.0);
    let (doc, _) = rows_with_shapes(100, view);

    // When
    let batches = batches(&doc, view);

    // Then: the backgrounds stay in batches over the lane's rows (one
    // row each here, as each row's shapes paint between them), and no
    // shape or mesh batch names a row
    let (shaped, rest): (Vec<_>, Vec<_>) = batches.iter().partition(|(shaped, _)| *shaped);
    assert!(rest.iter().any(|&&(_, rows)| rows > 0), "{batches:?}");
    assert!(shaped.len() >= 2, "{batches:?}");
    assert!(shaped.iter().all(|&&(_, rows)| rows == 0), "{batches:?}");
    assert_drawn_afresh(&doc, view, "shapes in rows");
}

#[test]
fn kept_batches_with_shapes_in_rows_match_after_a_scroll_and_an_insert() {
    // Given
    let view = viewport(400.0, 300.0);
    let (doc, list) = rows_with_shapes(100, view);
    assert_drawn_afresh(&doc, view, "first draw");

    // When: scrolled by 7 rows and a half, then a row inserted in view
    {
        let mut doc = doc.borrow_mut();
        let body = doc.body();
        assert!(doc.scroll_node_by(body, [0.0, 150.0]), "the body scrolls");
    }
    assert_drawn_afresh(&doc, view, "after the scroll");
    {
        let mut doc = doc.borrow_mut();
        let before = doc.children(list)[10];
        add_row(&mut doc, list, Some(before));
    }

    // Then
    assert_drawn_afresh(&doc, view, "after the insert");
}
