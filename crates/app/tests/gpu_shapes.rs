//! GPU tier: shape items drawn by the painter from its shape store
//! (docs/plans/one-scene.md S3a1): rects and circles batched in paint
//! order with quads; an unchanged frame writes no shape bytes; a shape
//! the store has no entry for refuses the frame, naming it. Elements
//! paint the items through `vapors_elements::test_items` until S4a's
//! `<path>` paints them.

#[path = "support/surface.rs"]
mod support;

use support::{BLUE, RED, Rig, WHITE, document, pixel, update};
use vapors_app::PaintError;
use vapors_elements::Document;
use vapors_elements::test_items::paint_after_background;
use vapors_scene::{Color, Item, Rect, Shape, ShapeRef};
use vapors_shapes::{Geometry, ShapeEntry, ShapeTable, TABLE_FORMAT, hash_bytes};

/// A rect from (0, 0) to (12, 8) and a circle of radius 6 about (6, 6),
/// both filled, as a table that lives for the test binary's life.
fn table() -> ShapeTable {
    let entry = |site, canonical: &'static [u8], geometry| ShapeEntry {
        site,
        canonical,
        hash: hash_bytes(canonical),
        geometry,
    };
    let rect = Geometry::Rect {
        origin: [0.0, 0.0],
        size: [12.0, 8.0],
        radius: [0.0, 0.0],
    };
    let circle = Geometry::Ellipse {
        centre: [6.0, 6.0],
        radii: [6.0, 6.0],
    };
    let entries = vec![
        entry("gpu_shapes.rs:1", b"Frect", rect),
        entry("gpu_shapes.rs:2", b"Fcircle", circle),
    ];
    ShapeTable::new(
        "gpu_shapes",
        TABLE_FORMAT,
        Box::leak(entries.into_boxed_slice()),
    )
}

/// Three empty boxes over the page, painted in order: `a`, `b` with a
/// blue background from (8, 0) to (24, 16), `c`.
const CSS: &str = "div { position: absolute; top: 0; left: 0; width: 32px; height: 32px; } \
    .b { left: 8px; width: 16px; height: 16px; background-color: #00f; }";

/// `shape` moved to `at` in red, its bounds `size` from there.
fn red(shape: u32, at: [f32; 2], size: [f32; 2]) -> Item {
    Item::Shape(Shape {
        shape: ShapeRef(shape),
        transform: [1.0, 0.0, 0.0, 1.0, at[0], at[1]],
        colour: Color::from_rgba8(255, 0, 0, 255),
        bounds: Rect::new(at[0], at[1], at[0] + size[0], at[1] + size[1]),
    })
}

/// The page with `a` painting the circle at (0, 2) then the rect at
/// (0, 20), and `c` the rect at (16, 4): two shape runs either side of
/// `b`'s quad.
fn page(c_shape: u32) -> Document {
    let mut doc = document(
        CSS,
        &[(None, "div", "a"), (None, "div", "b"), (None, "div", "c")],
    );
    let [a, _, c] = doc.children(doc.body())[..] else {
        panic!("three boxes")
    };
    let circle = red(1, [0.0, 2.0], [12.0, 12.0]);
    paint_after_background(a, vec![circle, red(0, [0.0, 20.0], [12.0, 8.0])]);
    paint_after_background(c, vec![red(c_shape, [16.0, 4.0], [12.0, 8.0])]);
    update(&mut doc, 1.0);
    doc
}

/// A painter with `table()` registered: the rect as shape 0, the circle 1.
fn rig() -> Rig {
    let mut rig = Rig::new();
    let refs = rig
        .painter
        .shapes_mut()
        .register(&table())
        .expect("registers");
    assert_eq!(refs, [ShapeRef(0), ShapeRef(1)]);
    rig
}

#[test]
fn gpu_shapes_are_drawn_in_paint_order_with_quads() {
    // Given
    let doc = page(0);
    let mut rig = rig();

    // When
    let pixels = rig.draw(&doc).expect("paints");

    // Then: the circle under b's quad, the rect over it
    assert_eq!(pixel(&pixels, 3, 8), RED, "the circle, left of b");
    assert_eq!(pixel(&pixels, 10, 8), BLUE, "b's quad over the circle");
    assert_eq!(pixel(&pixels, 20, 8), RED, "c's rect over b's quad");
    assert_eq!(pixel(&pixels, 4, 24), RED, "a's rect, its second shape");
    assert_eq!(pixel(&pixels, 30, 30), WHITE, "the page");
    let counts = rig.painter.frame_counts();
    assert_eq!(counts.shapes.draw_calls, 2, "a's two shapes are one run");
}

#[test]
fn gpu_an_unchanged_frame_writes_no_shape_bytes() {
    // Given
    let doc = page(0);
    let mut rig = rig();
    let first = rig.draw(&doc).expect("paints");
    assert!(rig.painter.frame_counts().shapes.buffer_bytes > 0);

    // When
    let again = rig.draw(&doc).expect("paints");

    // Then
    let counts = rig.painter.frame_counts();
    assert_eq!(counts.shapes.buffer_bytes, 0, "{counts:?}");
    assert_eq!(counts.shapes.draw_calls, 2);
    assert!(again == first, "the kept shapes draw the same pixels");
}

#[test]
fn gpu_a_shape_the_store_has_no_entry_for_is_refused_naming_it() {
    // Given: c names shape 5; two are registered
    let doc = page(5);
    let mut rig = rig();

    // When
    let refused = rig.draw(&doc).expect_err("refused");

    // Then
    assert!(matches!(refused, PaintError::Shape(_)), "{refused:?}");
    assert_eq!(
        refused.to_string(),
        "shape 5 is not registered in this painter's store, which holds 2; register its crate's table (ShapeStore::register) before drawing it"
    );
}

/// A 32 x 16 scroll box at the top of the page holding a 64 px tall
/// box that paints the rect at (0, 8) in the scroll frame.
const SCROLLER: &str = "div { position: absolute; top: 0; left: 0; } \
    .list { width: 32px; height: 16px; overflow: auto; } \
    .tall { position: static; width: 32px; height: 64px; }";

#[test]
fn gpu_a_shape_in_a_scrolled_box_moves_with_the_scroll() {
    // Given: the rect over rows 8 to 16, then scrolled 4 down
    let mut doc = document(SCROLLER, &[(None, "div", "list"), (Some(0), "div", "tall")]);
    let list = doc.children(doc.body())[0];
    let tall = doc.children(list)[0];
    paint_after_background(tall, vec![red(0, [0.0, 8.0], [12.0, 8.0])]);
    update(&mut doc, 1.0);
    let mut rig = rig();
    let before = rig.draw(&doc).expect("paints");
    assert!(doc.scroll_node_by(list, [0.0, 4.0]), "the list scrolls");
    update(&mut doc, 1.0);

    // When: drawn, the debug build's batch checks shifting every item
    let after = rig.draw(&doc).expect("paints");

    // Then: rows 4 to 12
    assert_eq!(pixel(&before, 4, 6), WHITE, "above the rect before");
    assert_eq!(pixel(&before, 4, 9), RED, "the rect before");
    assert_eq!(pixel(&after, 4, 5), RED, "the rect, 4 rows up");
    assert_eq!(pixel(&after, 4, 13), WHITE, "below it now");
}
