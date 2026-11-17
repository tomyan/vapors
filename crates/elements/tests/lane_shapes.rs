//! Shapes in the rows of a framed lane (`docs/plans/splice-layout.md`
//! section 11.2, 4f; `docs/plans/flat-scene-f2.md`, F2.3b): a step that
//! paints a shape or a mesh is batched over the lane's rows like any
//! other, each item naming its row, since every pipeline reads its row's
//! offset from the transform table. A page of rows each painting a
//! background, a rect and a path is one batch per pipeline. Kept batches
//! match a fresh batching after a scroll and an insert (the debug check
//! in `kept_batches`).

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

/// What a row's svg holds: each shape's tag and attributes.
type Shapes = &'static [(&'static str, &'static [(&'static str, &'static str)])];

/// A rect (a closed form) and a path (a mesh).
const BOTH: Shapes = &[("rect", RECT), ("path", PATH)];

/// A scrolling body holding a list of `count` 20 px rows, each with an
/// svg holding `shapes`, and a background when `class` is "row
/// coloured"; the document and the list.
fn rows_of(
    count: usize,
    (class, shapes): (&str, Shapes),
    view: Viewport,
) -> (Rc<RefCell<Document>>, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let list = element(&mut doc, body, "div", "list");
    for _ in 0..count {
        add_row(&mut doc, list, (class, shapes), None);
    }
    let css = "html { overflow: hidden; } body { margin: 0; overflow: auto; height: 300px; } \
               .row { height: 20px; } .coloured { background-color: #c33; } \
               .icon { display: block; }";
    doc.add_stylesheet("page.css", css).expect("parses");
    doc.update(view).expect("updates");
    (Rc::new(RefCell::new(doc)), list)
}

/// `count` rows, each with a background, a rect and a path.
fn rows_with_shapes(count: usize, view: Viewport) -> (Rc<RefCell<Document>>, NodeId) {
    rows_of(count, ("row coloured", BOTH), view)
}

/// A row of `class` holding an svg of `shapes`, put in `list` before
/// `before` (at the end when `None`).
fn add_row(
    doc: &mut Document,
    list: NodeId,
    (class, shapes): (&str, Shapes),
    before: Option<NodeId>,
) {
    let row = doc.create_element("div");
    doc.set_attribute(row, "class", class);
    doc.insert_before(list, row, before);
    let svg = element(doc, row, "svg", "icon");
    with_attributes(doc, svg, SVG);
    for &(tag, attributes) in shapes {
        let shape = element(doc, svg, tag, "");
        with_attributes(doc, shape, attributes);
    }
}

/// For each batch drawn at `view`: its run's kind, and how many
/// distinct rows of a lane its items name (0 for a batch in one frame).
fn batches(doc: &Rc<RefCell<Document>>, view: Viewport) -> Vec<(&'static str, usize)> {
    let mut doc = doc.borrow_mut();
    let painted = doc.update(view).expect("updates");
    let rect = vapors_scene::Rect::new(0.0, 0.0, view.width * view.scale, view.height * view.scale);
    let kept = painted.kept_batches(rect).clone();
    let kinds = kept.batched.batches.iter().map(|batch| kind(&batch.run));
    let rows = kept.rows.iter().map(|rows| {
        let named: std::collections::BTreeSet<_> = rows.items().iter().collect();
        named.len()
    });
    kinds.zip(rows).collect()
}

/// The kind of `run`, for the batches' list.
fn kind(run: &Run) -> &'static str {
    match run {
        Run::Quads(_) => "quads",
        Run::Shapes(_) => "shapes",
        Run::Meshes(_) => "meshes",
        _ => "other",
    }
}

/// The batches drawn at `view` over a lane's rows: each one's kind and
/// how many rows its items name.
fn lane_batches(doc: &Rc<RefCell<Document>>, view: Viewport) -> Vec<(&'static str, usize)> {
    batches(doc, view)
        .into_iter()
        .filter(|&(_, rows)| rows > 0)
        .collect()
}

#[test]
fn a_rect_in_each_lane_row_is_one_shape_batch_over_the_rows() {
    // Given: 100 rows, 15 in a 300 px view, each an svg holding a rect
    let view = viewport(400.0, 300.0);
    let (doc, _) = rows_of(100, ("row", &[("rect", RECT)]), view);

    // When
    let batches = lane_batches(&doc, view);

    // Then: one batch over the 15 rows in view (the 16th row touches
    // the view's bottom edge, its rect inset 2 px does not)
    assert_eq!(batches, [("shapes", 15)]);
    assert_drawn_afresh(&doc, view, "a rect in each row");
}

#[test]
fn a_path_in_each_lane_row_is_one_mesh_batch_over_the_rows() {
    // Given: 100 rows, 15 in a 300 px view, each an svg holding a path
    let view = viewport(400.0, 300.0);
    let (doc, _) = rows_of(100, ("row", &[("path", PATH)]), view);

    // When
    let batches = lane_batches(&doc, view);

    // Then
    assert_eq!(batches, [("meshes", 15)]);
    assert_drawn_afresh(&doc, view, "a path in each row");
}

#[test]
fn shapes_after_a_background_in_each_lane_row_name_their_rows_in_paint_order() {
    // Given: rows each with a background, a rect and a path
    let view = viewport(400.0, 300.0);
    let (doc, _) = rows_with_shapes(100, view);

    // When
    let batches = lane_batches(&doc, view);

    // Then: batches keep paint order (only consecutive items of a kind
    // join), so each of the 15 rows in view adds a quad, a shape and a
    // mesh batch, every one over the lane's rows, and the 16th, touching
    // the view's edge, its background; joining across rows would be
    // reordering (F13)
    let kinds: Vec<_> = batches.iter().map(|&(kind, _)| kind).collect();
    let row = ["quads", "shapes", "meshes"];
    assert_eq!(
        kinds,
        [row.repeat(15), vec!["quads"]].concat(),
        "{batches:?}"
    );
    assert_drawn_afresh(&doc, view, "shapes after backgrounds");
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
        add_row(&mut doc, list, ("row coloured", BOTH), Some(before));
    }

    // Then
    assert_drawn_afresh(&doc, view, "after the insert");
}
