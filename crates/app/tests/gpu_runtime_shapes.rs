//! GPU tier: a document's run-time shapes drawn by the painter (one
//! -scene S4a): acquired as an element paints, inside the update; a
//! repaint that keeps them tessellates and uploads nothing; and
//! elements mounted and removed a thousand times leave the cache's
//! entries, the store's bytes and the GPU buffer flat.

#[path = "support/surface.rs"]
mod support;

use support::{BLUE, RED, Rig, WHITE, document, pixel, update};
use vapors_dom::Dom;
use vapors_elements::Document;
use vapors_elements::test_items::{RuntimeItem, paint_shapes_after_background};
use vapors_scene::{Color, Rect};
use vapors_shapes::{FillRule, IconPath, Paint};

/// A red rect at (0, 0) and a blue triangle at (16, 0).
fn shapes() -> Vec<RuntimeItem> {
    [
        ("M0 0h12v8h-12z", [0.0, 0.0], RED),
        ("M0 0L12 0L0 12z", [16.0, 0.0], BLUE),
    ]
    .map(|(data, at, [r, g, b, a])| RuntimeItem {
        path: IconPath::parse(data).expect("parses"),
        paint: Paint::Fill(FillRule::NonZero),
        transform: [1.0, 0.0, 0.0, 1.0, at[0], at[1]],
        colour: Color::from_rgba8(r, g, b, a),
        bounds: Rect::new(at[0], at[1], at[0] + 12.0, at[1] + 12.0),
    })
    .to_vec()
}

/// A body holding one 32 px div `a` that paints [`shapes`].
fn page() -> Document {
    let mut doc = document(
        "div { position: absolute; width: 32px; height: 32px; } .shaded { background-color: #eeeeee; }",
        &[(None, "div", "a")],
    );
    let a = doc.children(doc.body())[0];
    paint_shapes_after_background(a, shapes());
    update(&mut doc, 1.0);
    doc
}

#[test]
fn gpu_a_documents_runtime_shapes_are_drawn() {
    // Given
    let doc = page();
    let mut rig = Rig::new();

    // When
    let pixels = rig.draw(&doc).expect("paints");

    // Then
    assert_eq!(pixel(&pixels, 4, 4), RED, "the run-time rect");
    assert_eq!(pixel(&pixels, 18, 2), BLUE, "the run-time triangle");
    assert_eq!(pixel(&pixels, 27, 10), WHITE, "past the triangle's slant");
    assert_eq!(
        doc.update_counts().shapes_tessellated,
        1,
        "the triangle alone"
    );
}

#[test]
fn gpu_a_repaint_that_keeps_its_shapes_tessellates_and_uploads_nothing() {
    // Given: the first frame drawn
    let mut doc = page();
    let mut rig = Rig::new();
    rig.draw(&doc).expect("paints");
    assert!(
        rig.painter.frame_counts().shape_bytes_uploaded > 0,
        "the triangle's bytes, once"
    );

    // When: the div repaints its background, acquiring its shapes again
    // inside the update
    let a = doc.children(doc.body())[0];
    doc.set_attribute(a, "class", "shaded");
    update(&mut doc, 1.0);
    let pixels = rig.draw(&doc).expect("paints");

    // Then
    assert_eq!(doc.update_counts().shapes_tessellated, 0);
    assert_eq!(rig.painter.frame_counts().shape_bytes_uploaded, 0);
    assert_eq!(pixel(&pixels, 18, 2), BLUE, "the triangle still drawn");
}

#[test]
fn gpu_elements_mounted_and_removed_a_thousand_times_leave_everything_flat() {
    // Given: one frame of the page, then a first mount and removal
    let mut doc = page();
    let mut rig = Rig::new();
    rig.draw(&doc).expect("paints");
    let cycle = |doc: &mut Document, rig: &mut Rig, at: u16| {
        let node = doc.create_element("div");
        paint_shapes_after_background(node, vec![own_shape(at)]);
        let body = doc.body();
        doc.append(body, node);
        update(doc, 1.0);
        rig.draw(doc).expect("paints");
        doc.remove(node);
        update(doc, 1.0);
        rig.draw(doc).expect("paints");
    };
    cycle(&mut doc, &mut rig, 0);
    let before = flat(&doc, &rig);

    // When: each mount paints a path no other does
    for at in 1..=1000 {
        cycle(&mut doc, &mut rig, at);
    }

    // Then
    assert_eq!(flat(&doc, &rig), before);
}

/// A triangle, a mesh no other mount paints: its corner moves with `at`.
fn own_shape(at: u16) -> RuntimeItem {
    let corner = 8.0 + f32::from(at) / 1000.0;
    RuntimeItem {
        path: IconPath::parse(&format!("M0 0L12 0L0 {corner}z")).expect("parses"),
        paint: Paint::Fill(FillRule::NonZero),
        transform: [1.0, 0.0, 0.0, 1.0, 0.0, 16.0],
        colour: Color::from_rgba8(0, 0, 0, 255),
        bounds: Rect::new(0.0, 16.0, 16.0, 32.0),
    }
}

/// The cache's live entries and slots, and the painter's retained
/// bytes, which hold the entry buffer.
fn flat(doc: &Document, rig: &Rig) -> (usize, usize, u64) {
    let counts = doc.update_counts();
    (
        counts.shapes_live,
        counts.shape_slots,
        rig.painter.frame_counts().retained_bytes,
    )
}
