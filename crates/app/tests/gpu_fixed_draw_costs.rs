//! GPU tier: the painter's fixed per-draw costs kept between frames
//! (docs/plans/iu4.md, PR 1): a viewport or blend-parameter uniform is
//! written when its value first appears, a pipeline's bind group is
//! made once, and an image's or surface's bind group is made with its
//! texture and dropped with it.

#[path = "support/surface.rs"]
mod support;

use support::{BLUE, RED, Rig, bare_document, pixel, update};
use vapors_app::FrameCounts;
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;
use vapors_scene::{RgbaImage, Size};

const SWATCH: &str = ".swatch { width: 8px; height: 8px; background-color: #0000ff; \
    box-shadow: 2px 2px 2px #000000; }";

/// A blue swatch with a shadow and a div holding text: a shadow run, a
/// quad run and a text run.
fn labelled() -> Document {
    let mut doc = bare_document(SWATCH, &[(None, "div", "swatch")]);
    let div = doc.create_element("div");
    let body = doc.body();
    doc.append(body, div);
    let text = doc.create_text("Ab");
    doc.append(div, text);
    update(&mut doc, 1.0);
    doc
}

/// A document showing `pic.png`, a 4 x 2 image of `colour`, over 4 x 2 px.
fn with_image(colour: [u8; 4]) -> (Document, NodeId) {
    let mut doc = bare_document("img { display: block; width: 4px; height: 2px; }", &[]);
    let img = doc.create_element("img");
    doc.set_attribute(img, "src", "pic.png");
    let body = doc.body();
    doc.append(body, img);
    doc.register_image("pic.png", solid(colour))
        .expect("registers");
    update(&mut doc, 1.0);
    (doc, img)
}

fn solid(colour: [u8; 4]) -> RgbaImage {
    RgbaImage {
        size: Size {
            width: 4,
            height: 2,
        },
        rgba: colour.repeat(4 * 2),
    }
}

fn drawn(rig: &mut Rig, doc: &Document) -> FrameCounts {
    rig.draw(doc).expect("draws");
    rig.painter.frame_counts()
}

fn drawn_at(rig: &mut Rig, doc: &Document, width: u32, height: u32) -> FrameCounts {
    rig.draw_at(doc, Size { width, height }).expect("draws");
    rig.painter.frame_counts()
}

#[test]
fn gpu_an_idle_frame_makes_no_bind_group_and_writes_no_uniform() {
    // Given
    let doc = labelled();
    let mut rig = Rig::new();
    let first = drawn(&mut rig, &doc);

    // When
    let idle = drawn(&mut rig, &doc);

    // Then: shadows, quads and text each made their bind group and wrote their uniform once
    assert_eq!((first.bind_groups_made(), first.uniform_writes()), (3, 3));
    assert_eq!((idle.bind_groups_made(), idle.uniform_writes()), (0, 0));
    assert_eq!(idle.bind_groups_kept, 3);
}

#[test]
fn gpu_a_resize_writes_each_pipelines_viewport_once_and_no_bind_group() {
    // Given
    let doc = labelled();
    let mut rig = Rig::new();
    drawn(&mut rig, &doc);

    // When: the target is one pixel taller, then back
    let taller = drawn_at(&mut rig, &doc, 32, 33);
    let back = drawn_at(&mut rig, &doc, 32, 32);

    // Then: the text's blend parameters do not depend on the viewport
    assert_eq!(taller.quads.uniform_writes, 1);
    assert_eq!(taller.shadows.uniform_writes, 1);
    assert_eq!(taller.texts.uniform_writes, 0);
    assert_eq!(taller.bind_groups_made(), 0);
    assert_eq!((back.bind_groups_made(), back.uniform_writes()), (0, 0));
}

#[test]
fn gpu_an_image_registered_again_gets_a_new_bind_group_and_draws_its_pixels() {
    // Given: pic.png red, drawn twice
    let (mut doc, _) = with_image(RED);
    let mut rig = Rig::new();
    drawn(&mut rig, &doc);
    let idle = drawn(&mut rig, &doc);

    // When: pic.png is blue
    doc.register_image("pic.png", solid(BLUE))
        .expect("registers");
    update(&mut doc, 1.0);
    let rgba = rig.draw(&doc).expect("draws");
    let replaced = rig.painter.frame_counts();

    // Then: one binding, for the new texture, and the old one gone
    assert_eq!(idle.images.bind_groups_made, 0);
    assert_eq!(replaced.images.bind_groups_made, 1);
    assert_eq!(pixel(&rgba, 1, 1), BLUE);
    assert_eq!((idle.bind_groups_kept, replaced.bind_groups_kept), (1, 1));
}

#[test]
fn gpu_a_freed_textures_bind_group_is_dropped_with_it() {
    // Given: an image drawn
    let (mut doc, img) = with_image(RED);
    let mut rig = Rig::new();
    let shown = drawn(&mut rig, &doc);

    // When: the img is gone and a frame is drawn without it
    doc.remove(img);
    update(&mut doc, 1.0);
    let gone = drawn(&mut rig, &doc);

    // Then
    assert_eq!(shown.bind_groups_kept, 1);
    assert_eq!(gone.bind_groups_kept, 0);
}
