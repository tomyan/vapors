//! GPU tier: a frame's clip on the clip table (`docs/plans/frame-clips.md`,
//! slice 1). With every scissor widened to the whole target, a scroll
//! frame's rows past its clip are still cut: those with a clip of their
//! own through their entry's parent, the frame's entry, and those with
//! none through the frame's entry itself. The frame's entry is on the
//! page, so a scrolled list, and a list in a scrolled one, cut at their
//! edges as they show.

use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};
use vapors_gpu_harness::gpu;
use vapors_renderer::{FrameEncoder, Target, read_rgba8};
use vapors_scene::Size;
use vapors_text::FontSystem;

use crate::ScenePainter;

const SIDE: u32 = 16;

const CSS: &str = "body { margin: 0; background-color: #fff; } \
    .outer { overflow: auto; width: 16px; height: 12px; } \
    .spacer { height: 4px; } \
    .list { overflow: auto; width: 16px; height: 8px; } \
    .row { height: 6px; background-color: #f00; } \
    .cut { overflow: hidden; height: 6px; } \
    .wide { width: 16px; height: 12px; background-color: #00f; }";

/// A white page holding an 8 px tall list, 16 px wide, of 6 px rows of
/// `classes`: `row`, red with no clip of its own; `cut`, a row clipping a
/// blue box taller than it. When `nested`, the list is between two 4 px
/// spacers in a 12 px tall scroller, which can scroll 4 px. The list and
/// the scroller it is in.
fn page(classes: &[&str], nested: bool) -> (Document, NodeId, Option<NodeId>) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet("app.css", CSS).expect("parses");
    let body = doc.body();
    let outer = nested.then(|| {
        let outer = child(&mut doc, body, "outer");
        child(&mut doc, outer, "spacer");
        outer
    });
    let list = child(&mut doc, outer.unwrap_or(body), "list");
    for &class in classes {
        let row = child(&mut doc, list, class);
        if class == "cut" {
            child(&mut doc, row, "wide");
        }
    }
    if let Some(outer) = outer {
        child(&mut doc, outer, "spacer");
    }
    (doc, list, outer)
}

/// A div of `class` appended to `parent`.
fn child(doc: &mut Document, parent: NodeId, class: &str) -> NodeId {
    let node = doc.create_element("div");
    doc.set_attribute(node, "class", class);
    doc.append(parent, node);
    node
}

/// `doc` updated on the 16 px square; how many lanes it framed.
fn updated(doc: &mut Document) -> usize {
    let side = f32::from(u16::try_from(SIDE).expect("small"));
    let view = Viewport {
        width: side,
        height: side,
        scale: 1.0,
    };
    doc.update(view).expect("updates");
    doc.update_counts().lanes_framed
}

/// `doc`'s paint drawn on a fresh square, every scissor widened when
/// `widened`.
fn drawn(doc: &Document, widened: bool) -> Vec<u8> {
    let gpu = gpu();
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("frame clips"),
        size: wgpu::Extent3d {
            width: SIDE,
            height: SIDE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let encoder = FrameEncoder::new(&gpu.device);
    let target = Target {
        device: &gpu.device,
        queue: &gpu.queue,
        encoder: &encoder,
        view: &view,
        multisampled: None,
        viewport: Size {
            width: SIDE,
            height: SIDE,
        },
        scissor: None,
    };
    let mut painter = ScenePainter::new(&gpu.device, format).expect("a supported format");
    painter.scissor_widened = widened;
    let shown = doc.painted().expect("painted");
    painter.draw(&target, shown).expect("draws");
    encoder.submit(&gpu.queue).expect("no refusal");
    read_rgba8(&gpu.device, &gpu.queue, &texture)
}

/// `doc` drawn under the scissor and with it widened: both.
fn both(doc: &Document) -> (Vec<u8>, Vec<u8>) {
    (drawn(doc, false), drawn(doc, true))
}

/// The pixel at `x`, `y` of `pixels`.
fn at(pixels: &[u8], x: usize, y: usize) -> [u8; 4] {
    let i = (y * SIDE as usize + x) * 4;
    [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
}

const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];

#[test]
fn gpu_a_row_clip_crossing_a_scroll_frames_edge_is_cut_by_the_frames_entry() {
    // Given: the clipping row crosses the list's bottom, y 8
    let (mut doc, ..) = page(&["row", "cut"], false);
    updated(&mut doc);

    // When
    let (scissored, widened) = both(&doc);

    // Then: the blue box ends at y 8 without the scissor, its row's entry
    // under the frame's
    assert_eq!((at(&widened, 4, 7), at(&widened, 4, 9)), (BLUE, WHITE));
    assert!(widened == scissored, "the table cuts as the scissor does");
}

#[test]
fn gpu_a_row_with_no_clip_crossing_a_scroll_frames_edge_is_cut_by_the_frames_entry() {
    // Given: the red row, with no clip of its own, crosses the bottom
    let (mut doc, ..) = page(&["cut", "row"], false);
    updated(&mut doc);

    // When
    let (scissored, widened) = both(&doc);

    // Then: it ends at y 8 without the scissor, its entry the frame's
    assert_eq!((at(&widened, 4, 7), at(&widened, 4, 9)), (RED, WHITE));
    assert!(widened == scissored, "the table cuts as the scissor does");
}

#[test]
fn gpu_a_scrolled_frames_entry_cuts_at_its_edge_on_the_page() {
    // Given: the list scrolled 2 px, its second row, a clipping one, at
    // y 4..10 crossing the list's bottom
    let (mut doc, list, _) = page(&["row", "cut", "row"], false);
    updated(&mut doc);
    assert!(doc.scroll_node_by(list, [0.0, 2.0]), "the list scrolls");
    updated(&mut doc);

    // When
    let (scissored, widened) = both(&doc);

    // Then: cut at y 8 where the list's bottom is on the page; an entry
    // under the list's own transform would cut 2 px further up
    assert_eq!((at(&widened, 4, 7), at(&widened, 4, 9)), (BLUE, WHITE));
    assert!(widened == scissored, "the table cuts as the scissor does");
}

#[test]
fn gpu_a_list_in_a_scrolled_frame_cuts_at_its_edge_on_the_page() {
    // Given: the scroller scrolled 2 px, so the list is at y 2..10, its
    // clipping row at y 8..14, and the scroller's own bottom at y 12
    let (mut doc, _, outer) = page(&["row", "cut"], true);
    updated(&mut doc);
    let outer = outer.expect("nested");
    assert!(
        doc.scroll_node_by(outer, [0.0, 2.0]),
        "the scroller scrolls"
    );
    updated(&mut doc);

    // When
    let (scissored, widened) = both(&doc);

    // Then: the blue box ends at the list's bottom on the page, y 10, not
    // the scroller's, the list's entry set where its clip now is
    assert_eq!((at(&widened, 4, 9), at(&widened, 4, 11)), (BLUE, WHITE));
    assert!(widened == scissored, "the table cuts as the scissor does");
}

#[test]
fn gpu_a_framed_lanes_rows_are_cut_by_the_lane_clips_entry() {
    // Given: 40 rows, so the list frames them as a lane, the second a
    // clipping row crossing the list's bottom
    let mut classes = vec!["row", "cut"];
    classes.resize(40, "row");
    let (mut doc, ..) = page(&classes, false);

    // When
    let framed = updated(&mut doc);
    let (scissored, widened) = both(&doc);

    // Then: the list framed its rows; the row's entry, its clip moved into
    // the lane's coordinates, is under the lane's clip entry
    assert_eq!(framed, 1, "the list is a framed lane");
    assert_eq!((at(&widened, 4, 7), at(&widened, 4, 9)), (BLUE, WHITE));
    assert!(widened == scissored, "the table cuts as the scissor does");
}

/// A white page holding a 16 px square card with 6 px corners that clips
/// a list of 40 red 6 px rows: a lane, framed, under the card's rounded
/// clip (`docs/plans/frame-clips.md`, slice 2).
fn rounded_lane() -> Document {
    let mut doc = Document::new(FontSystem::bundle_only());
    let css = "body { margin: 0; background-color: #fff; } \
        .card { overflow: hidden; width: 16px; height: 16px; border-radius: 6px; } \
        .row { height: 6px; background-color: #f00; }";
    doc.add_stylesheet("app.css", css).expect("parses");
    let body = doc.body();
    let card = child(&mut doc, body, "card");
    let list = child(&mut doc, card, "list");
    for _ in 0..40 {
        child(&mut doc, list, "row");
    }
    doc
}

#[test]
fn gpu_a_framed_lanes_rows_under_a_rounded_clip_are_cut_by_its_curve() {
    // Given
    let mut doc = rounded_lane();
    let framed = updated(&mut doc);

    // When
    let (scissored, widened) = both(&doc);

    // Then: the rows are framed, and the corner pixel, outside the 6 px
    // curve, is the page's, with or without the scissor; inside, red
    assert_eq!(framed, 1, "the list is a framed lane");
    assert_eq!((at(&widened, 0, 0), at(&widened, 8, 3)), (WHITE, RED));
    assert!(widened == scissored, "the table cuts as the scissor does");
}
