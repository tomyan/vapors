//! GPU tier: shadow, quad, image and surface batches drawn from bytes
//! kept between frames (IU4 PR 2, `painter_kept.rs`): an unchanged frame
//! writes none; an edit writes only the batches whose items changed; a
//! batch no longer drawn has its bytes released; image quads kept for
//! one viewport are written again for another with the same view; a
//! box whose edge rounds to -0.0 and then to 0.0 has its batch written
//! again (its bytes differ in the sign bit).

#[path = "support/surface.rs"]
mod support;

use support::{BLUE, RED, Rig, WHITE, document, pixel, update};
use vapors_app::FrameCounts;
use vapors_dom::Dom;
use vapors_elements::Document;
use vapors_gpu_harness::read_rgba8;
use vapors_renderer::{FrameEncoder, Target, clear};
use vapors_scene::{Color, RgbaImage, Scissor, Size};

/// A shadowed red box, a red image and a red box, left to right.
const CSS: &str = "div, img { position: absolute; top: 0; width: 8px; height: 8px; } \
    .a { left: 0; background-color: #f00; box-shadow: 0 0 2px 0 #000; } \
    img { display: block; left: 12px; } .c { left: 24px; background-color: #f00; } \
    .blue { background-color: #00f; } .gone { display: none; }";

/// The page of `CSS`, its image registered, updated.
fn page() -> Document {
    let mut doc = document(
        CSS,
        &[(None, "div", "a"), (None, "img", ""), (None, "div", "c")],
    );
    let img = doc.children(doc.body())[1];
    doc.set_attribute(img, "src", "red.png");
    let size = Size {
        width: 1,
        height: 1,
    };
    doc.register_image(
        "red.png",
        RgbaImage {
            size,
            rgba: RED.to_vec(),
        },
    )
    .expect("registers");
    update(&mut doc, 1.0);
    doc
}

/// `doc` drawn by `rig`: the pixels and the frame's counts.
fn drawn(rig: &mut Rig, doc: &Document) -> (Vec<u8>, FrameCounts) {
    let pixels = rig.draw(doc).expect("paints");
    (pixels, rig.painter.frame_counts())
}

/// The bytes written for shadow, quad, image and surface runs.
fn kept_kinds_bytes(counts: &FrameCounts) -> u64 {
    let kinds = [counts.shadows, counts.quads, counts.images, counts.surfaces];
    kinds.iter().map(|work| work.buffer_bytes).sum()
}

/// Sets `class` on the `at`th element of `doc`'s body and updates it.
fn classed(doc: &mut Document, at: usize, class: &str) {
    let element = doc.children(doc.body())[at];
    doc.set_attribute(element, "class", class);
    update(doc, 1.0);
}

#[test]
fn gpu_an_unchanged_frame_writes_no_shadow_quad_or_image_bytes() {
    // Given
    let doc = page();
    let mut rig = Rig::new();
    let (first, written) = drawn(&mut rig, &doc);

    // When
    let (again, counts) = drawn(&mut rig, &doc);

    // Then
    assert_eq!((written.batches_written, written.batches_kept), (5, 0));
    assert_eq!((counts.batches_written, counts.batches_kept), (0, 5));
    assert_eq!(kept_kinds_bytes(&counts), 0, "{counts:?}");
    assert_eq!(counts.retained_bytes, written.retained_bytes);
    assert_eq!(again, first);
    assert_eq!([pixel(&again, 4, 4), pixel(&again, 14, 4)], [RED, RED]);
}

#[test]
fn gpu_a_box_whose_background_changes_writes_only_its_batch() {
    // Given
    let mut doc = page();
    let mut rig = Rig::new();
    drawn(&mut rig, &doc);

    // When: the last box turns blue
    classed(&mut doc, 2, "c blue");
    let (pixels, counts) = drawn(&mut rig, &doc);

    // Then
    assert_eq!((counts.batches_written, counts.batches_kept), (1, 4));
    assert_eq!(counts.quads.buffer_bytes, 128);
    assert_eq!([pixel(&pixels, 4, 4), pixel(&pixels, 28, 4)], [RED, BLUE]);
}

#[test]
fn gpu_a_batch_no_longer_drawn_has_its_bytes_released() {
    // Given
    let mut doc = page();
    let mut rig = Rig::new();
    let (_, before) = drawn(&mut rig, &doc);

    // When: the last box is not displayed
    classed(&mut doc, 2, "c gone");
    let (pixels, counts) = drawn(&mut rig, &doc);

    // Then: its quad's 128 bytes are gone
    assert_eq!(counts.retained_bytes, before.retained_bytes - 128);
    assert_eq!(pixel(&pixels, 28, 4), WHITE);
}

#[test]
fn gpu_image_quads_kept_for_one_viewport_draw_on_another_with_the_same_view_unwritten() {
    // Given: drawn on a 64 px target scissored to its top-left 32 px
    let doc = page();
    let mut rig = Rig::new();
    let scissor = Scissor {
        x: 0,
        y: 0,
        width: 32,
        height: 32,
    };
    draw_on(&mut rig, &doc, (64, Some(scissor)));

    // When: drawn on a 32 px target, the same view
    let pixels = draw_on(&mut rig, &doc, (32, None));

    // Then: the image's quad, in device pixels, is drawn as kept; the
    // 16 bytes are the new viewport's uniform value
    let counts = rig.painter.frame_counts();
    assert_eq!(counts.batches_written, 0, "{counts:?}");
    let images = counts.images;
    assert_eq!((images.buffer_bytes, images.uniform_writes), (16, 1));
    let at = |x: u32, y: u32| pixels[((y * 32 + x) * 4) as usize..][..4].to_vec();
    assert_eq!([at(14, 4), at(20, 4)], [RED.to_vec(), WHITE.to_vec()]);
}

#[test]
fn gpu_a_box_moved_from_an_edge_rounding_to_minus_zero_to_zero_is_written_again() {
    // Given: a red box whose top, -0.3 px, rounds to -0.0
    let css = "div { position: absolute; left: 0; top: -0.3px; width: 8px; height: 8px; \
               background-color: #f00; } .zero { top: 0; }";
    let mut doc = document(css, &[(None, "div", "")]);
    update(&mut doc, 1.0);
    let mut rig = Rig::new();
    drawn(&mut rig, &doc);

    // When: moved to 0, which draws the same pixels
    classed(&mut doc, 0, "zero");
    let (pixels, counts) = drawn(&mut rig, &doc);

    // Then: written again, with no debug compare panic
    assert_eq!((counts.batches_written, counts.batches_kept), (1, 0));
    assert_eq!(pixel(&pixels, 4, 4), RED);
}

/// `doc` drawn on a target `side` px square under `scissor`: its pixels.
fn draw_on(rig: &mut Rig, doc: &Document, (side, scissor): (u32, Option<Scissor>)) -> Vec<u8> {
    let gpu = &rig.gpu;
    let texture = clear(&gpu.device, &gpu.queue, side, side, Color::default());
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let encoder = FrameEncoder::new(&gpu.device);
    let target = Target {
        device: &gpu.device,
        queue: &gpu.queue,
        encoder: &encoder,
        view: &view,
        viewport: Size {
            width: side,
            height: side,
        },
        scissor,
    };
    let painted = doc.painted().expect("updated");
    rig.painter.draw(&target, painted).expect("paints");
    encoder.submit(&gpu.queue).expect("no refusal");
    read_rgba8(&gpu.device, &gpu.queue, &texture)
}
