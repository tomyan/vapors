//! GPU tier: `ScenePainter::frame_counts` (IU1 PR 3), the bytes the last
//! frame wrote to the GPU and the draw calls it issued, by run kind:
//! counted where the painter's pipelines issue them, with no readback.

#[path = "support/surface.rs"]
mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::{RED, Rig, Solid, app_frame, bare_document, update};
use vapors_app::{ElementsApp, FrameCounts};
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;
use vapors_scene::{RgbaImage, Size};
use vapors_text::AtlasConfig;

/// One instance (8 four-float attributes) and the viewport uniform.
const ONE_QUAD_BYTES: u64 = 8 * 16 + 16;
/// One shadow instance (7 four-float attributes) and the viewport.
const ONE_SHADOW_BYTES: u64 = 7 * 16 + 16;
/// The atlas page, one coverage byte per pixel, uploaded whole.
const PAGE_BYTES: u64 = 1024 * 1024;
/// The coverage bitmap of the default-size "c" in the test font, one
/// byte per pixel, as the atlas stored it when measured.
const C_GLYPH_BYTES: u64 = 72;
/// An image quad: 6 vertices of 4 floats.
const IMAGE_QUAD_BYTES: u64 = 6 * 16;
/// The image pipeline's viewport and shift uniform, written on its first draw.
const VIEW_UNIFORM_BYTES: u64 = 16;

const SWATCH: &str = ".swatch { width: 8px; height: 8px; background-color: #0000ff; \
    box-shadow: 2px 2px 2px #000000; }";

/// A blue swatch with a shadow and, after it, a div holding `label`'s text node.
fn labelled(label: &str) -> (Document, NodeId) {
    let mut doc = bare_document(SWATCH, &[(None, "div", "swatch")]);
    let div = doc.create_element("div");
    let body = doc.body();
    doc.append(body, div);
    let text = doc.create_text(label);
    doc.append(div, text);
    update(&mut doc, 1.0);
    (doc, text)
}

/// `doc` drawn by `rig`, and the counts of that frame.
fn drawn(rig: &mut Rig, doc: &Document) -> FrameCounts {
    rig.draw(doc).expect("draws");
    rig.painter.frame_counts()
}

#[test]
fn gpu_a_frame_with_nothing_changed_uploads_no_atlas_bytes() {
    // Given
    let (doc, _) = labelled("Ab");
    let mut rig = Rig::new();
    let first = drawn(&mut rig, &doc);

    // When
    let idle = drawn(&mut rig, &doc);

    // Then
    assert_eq!(first.atlas_bytes(), PAGE_BYTES);
    assert_eq!(idle.atlas_bytes(), 0);
    assert_eq!(idle.texts.draw_calls, 1);
}

#[test]
fn gpu_a_frame_adding_one_glyph_uploads_exactly_that_glyphs_bytes() {
    // Given
    let (mut doc, text) = labelled("Ab");
    let mut rig = Rig::new();
    drawn(&mut rig, &doc);

    // When
    doc.set_text(text, "Abc");
    update(&mut doc, 1.0);
    let changed = drawn(&mut rig, &doc);

    // Then
    assert_eq!(changed.atlas_bytes(), C_GLYPH_BYTES);
    assert_eq!(AtlasConfig::default().page_size, 1024);
}

#[test]
fn gpu_draw_calls_are_one_per_quad_shadow_or_text_run() {
    // Given
    let (doc, _) = labelled("Ab");
    let mut rig = Rig::new();

    // When
    let counts = drawn(&mut rig, &doc);

    // Then
    assert_eq!(counts.quads.draw_calls, 1);
    assert_eq!(counts.quads.buffer_bytes, ONE_QUAD_BYTES);
    assert_eq!(counts.shadows.draw_calls, 1);
    assert_eq!(counts.shadows.buffer_bytes, ONE_SHADOW_BYTES);
    assert_eq!(counts.texts.draw_calls, 1);
    assert_eq!(counts.total().draw_calls, 3);
}

#[test]
fn gpu_two_text_items_in_one_run_are_one_draw() {
    // Given
    let (mut doc, _) = labelled("Ab");
    let div = doc.create_element("div");
    let body = doc.body();
    doc.append(body, div);
    let second = doc.create_text("Cd");
    doc.append(div, second);
    update(&mut doc, 1.0);
    let mut rig = Rig::new();

    // When
    let counts = drawn(&mut rig, &doc);

    // Then
    assert_eq!(counts.texts.draw_calls, 1);
}

#[test]
fn gpu_an_elements_app_reports_its_painters_last_frame() {
    // Given
    let (doc, _) = labelled("Ab");
    let mut app = ElementsApp::new(Rc::new(RefCell::new(doc)));
    assert_eq!(app.frame_counts(), FrameCounts::default());

    // When
    app_frame(&mut app, &vapors_gpu_harness::gpu());

    // Then
    assert_eq!(app.frame_counts().total().draw_calls, 3);
}

#[test]
fn gpu_a_surface_frame_counts_its_composition() {
    // Given
    let css = ".track { width: 16px; height: 16px; }";
    let mut doc = bare_document(css, &[(None, "gpu-surface", "track")]);
    update(&mut doc, 1.0);
    let mut rig = Rig::new();
    let (solid, _) = Solid::new(RED);
    rig.painter.mount("track", Box::new(solid)).expect("mounts");

    // When
    let counts = drawn(&mut rig, &doc);

    // Then
    assert_eq!(counts.surfaces.draw_calls, 1);
    assert_eq!(
        counts.surfaces.buffer_bytes,
        IMAGE_QUAD_BYTES + VIEW_UNIFORM_BYTES
    );
    assert_eq!(counts.total().draw_calls, 1);
}

#[test]
fn gpu_an_image_uploads_every_mip_level_once() {
    // Given: a 4 x 2 image, levels 4 x 2, 2 x 1 and 1 x 1
    let mut doc = bare_document("img { display: block; width: 4px; height: 2px; }", &[]);
    let img = doc.create_element("img");
    doc.set_attribute(img, "src", "pic.png");
    let body = doc.body();
    doc.append(body, img);
    let size = Size {
        width: 4,
        height: 2,
    };
    let rgba = vec![255; 4 * 2 * 4];
    doc.register_image("pic.png", RgbaImage { size, rgba })
        .expect("registers");
    update(&mut doc, 1.0);
    let mut rig = Rig::new();

    // When
    let first = drawn(&mut rig, &doc);
    let again = drawn(&mut rig, &doc);

    // Then
    assert_eq!(first.images.texture_bytes, (8 + 2 + 1) * 4);
    assert_eq!(first.images.draw_calls, 1);
    assert_eq!(again.images.texture_bytes, 0);
}
