//! GPU tier: a document's run-time shapes drawn by the painter (one
//! -scene S4a): the store takes them from the document's frame, each
//! mesh's triangles uploaded once, and an update that paints no new
//! path tessellates nothing and a frame drawing nothing new uploads
//! nothing.

#[path = "support/surface.rs"]
mod support;

use support::{BLUE, RED, Rig, WHITE, document, pixel, update};
use vapors_elements::test_items::{paint_after_background, runtime_item};
use vapors_scene::{Color, Item, Rect};
use vapors_shapes::{FillRule, IconPath, Paint};

const FILL: Paint = Paint::Fill(FillRule::NonZero);

/// A body holding one 32 px div that paints, after its background, a
/// red run-time rect at (0, 0) and a blue run-time triangle at (16, 0).
fn page() -> vapors_elements::Document {
    let mut doc = document(
        "div { position: absolute; width: 32px; height: 32px; }",
        &[(None, "div", "a")],
    );
    let a = doc.children(doc.body())[0];
    let items = [
        ("M0 0h12v8h-12z", [0.0, 0.0], RED),
        ("M0 0L12 0L0 12z", [16.0, 0.0], BLUE),
    ]
    .map(|(data, at, colour)| shape(&mut doc, data, at, colour));
    paint_after_background(a, items.to_vec());
    update(&mut doc, 1.0);
    doc
}

/// `data` as a run-time shape of `doc` moved to `at` in `colour`.
fn shape(doc: &mut vapors_elements::Document, data: &str, at: [f32; 2], colour: [u8; 4]) -> Item {
    let path = IconPath::parse(data).expect("parses");
    let colour = Color::from_rgba8(colour[0], colour[1], colour[2], colour[3]);
    let bounds = Rect::new(at[0], at[1], at[0] + 12.0, at[1] + 12.0);
    runtime_item(
        doc,
        (&path, FILL),
        [1.0, 0.0, 0.0, 1.0, at[0], at[1]],
        colour,
        bounds,
    )
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
fn gpu_an_unchanged_frame_tessellates_and_uploads_nothing() {
    // Given: the first frame drawn
    let mut doc = page();
    let mut rig = Rig::new();
    rig.draw(&doc).expect("paints");
    let first = rig.painter.frame_counts().shape_bytes_uploaded;

    // When
    update(&mut doc, 1.0);
    rig.draw(&doc).expect("paints");

    // Then
    assert!(first > 0, "the triangle's bytes, once");
    assert_eq!(doc.update_counts().shapes_tessellated, 0);
    assert_eq!(rig.painter.frame_counts().shape_bytes_uploaded, 0);
}
