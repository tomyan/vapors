//! GPU tier: a frame's clip on the clip table (`docs/plans/frame-clips.md`,
//! slice 1). With every scissor widened to the whole target, a scroll
//! frame's rows past its clip are still cut: those with a clip of their
//! own through their entry's parent, the frame's entry, and those with
//! none through the frame's entry itself.

use vapors_dom::Dom;
use vapors_elements::{Document, Viewport};
use vapors_gpu_harness::gpu;
use vapors_renderer::{FrameEncoder, Target, read_rgba8};
use vapors_scene::Size;
use vapors_text::FontSystem;

use crate::ScenePainter;

const SIDE: u32 = 16;

/// An 8 px tall list over a white page holding 6 px rows of `classes`:
/// `row`, red with no clip of its own; `cut`, a row clipping a blue box
/// taller than it. The second row crosses the list's bottom, y 8; a row
/// wholly below it is culled, never drawn, so shows no cut.
fn page(classes: &[&str]) -> Document {
    let mut doc = Document::new(FontSystem::bundle_only());
    let css = "body { margin: 0; background-color: #fff; } \
        .list { overflow: auto; width: 16px; height: 8px; } \
        .row { height: 6px; background-color: #f00; } \
        .cut { overflow: hidden; height: 6px; } \
        .wide { width: 16px; height: 12px; background-color: #00f; }";
    doc.add_stylesheet("app.css", css).expect("parses");
    let list = doc.create_element("div");
    doc.set_attribute(list, "class", "list");
    doc.append(doc.body(), list);
    for &class in classes {
        let row = doc.create_element("div");
        doc.set_attribute(row, "class", class);
        doc.append(list, row);
        if class == "cut" {
            let wide = doc.create_element("div");
            doc.set_attribute(wide, "class", "wide");
            doc.append(row, wide);
        }
    }
    doc
}

/// The page of `classes` drawn on a fresh square, every scissor widened
/// when `widened`; and how many lanes its update framed.
fn drawn(classes: &[&str], widened: bool) -> (Vec<u8>, usize) {
    let mut doc = page(classes);
    let side = f32::from(u16::try_from(SIDE).expect("small"));
    let view = Viewport {
        width: side,
        height: side,
        scale: 1.0,
    };
    doc.update(view).expect("updates");
    let framed = doc.update_counts().lanes_framed;
    let shown = doc.painted().expect("painted");
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
    let target_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let encoder = FrameEncoder::new(&gpu.device);
    let target = Target {
        device: &gpu.device,
        queue: &gpu.queue,
        encoder: &encoder,
        view: &target_view,
        multisampled: None,
        viewport: Size {
            width: SIDE,
            height: SIDE,
        },
        scissor: None,
    };
    let mut painter = ScenePainter::new(&gpu.device, format).expect("a supported format");
    painter.scissor_widened = widened;
    painter.draw(&target, shown).expect("draws");
    encoder.submit(&gpu.queue).expect("no refusal");
    (read_rgba8(&gpu.device, &gpu.queue, &texture), framed)
}

/// The pixel at `x`, `y` of `pixels`.
fn at(pixels: &[u8], x: usize, y: usize) -> [u8; 4] {
    let i = (y * SIDE as usize + x) * 4;
    [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
}

#[test]
fn gpu_a_row_clip_crossing_a_scroll_frames_edge_is_cut_by_the_frames_entry() {
    // When: the clipping row crosses the list's bottom
    let classes = &["row", "cut"];
    let ((cut_by_scissor, _), (widened, _)) = (drawn(classes, false), drawn(classes, true));

    // Then: the blue box ends at y 8 without the scissor, its row's entry
    // under the frame's
    assert_eq!(at(&widened, 4, 7), [0, 0, 255, 255]);
    assert_eq!(at(&widened, 4, 9), [255, 255, 255, 255]);
    assert!(
        widened == cut_by_scissor,
        "the table cuts as the scissor does"
    );
}

#[test]
fn gpu_a_row_with_no_clip_crossing_a_scroll_frames_edge_is_cut_by_the_frames_entry() {
    // When: the red row, with no clip of its own, crosses the bottom
    let classes = &["cut", "row"];
    let ((cut_by_scissor, _), (widened, _)) = (drawn(classes, false), drawn(classes, true));

    // Then: it ends at y 8 without the scissor, its entry the frame's
    assert_eq!(at(&widened, 4, 7), [255, 0, 0, 255]);
    assert_eq!(at(&widened, 4, 9), [255, 255, 255, 255]);
    assert!(
        widened == cut_by_scissor,
        "the table cuts as the scissor does"
    );
}

#[test]
fn gpu_a_framed_lanes_rows_are_cut_by_the_lane_clips_entry() {
    // Given: 40 rows, so the list frames them as a lane, the second a
    // clipping row crossing the list's bottom
    let mut classes = vec!["row", "cut"];
    classes.resize(40, "row");

    // When
    let ((cut_by_scissor, framed), (widened, _)) = (drawn(&classes, false), drawn(&classes, true));

    // Then: the list framed its rows; the row's entry, in its row's
    // transform, is under the lane's clip entry, on the page
    assert_eq!(framed, 1, "the list is a framed lane");
    assert_eq!(at(&widened, 4, 7), [0, 0, 255, 255]);
    assert_eq!(at(&widened, 4, 9), [255, 255, 255, 255]);
    assert!(
        widened == cut_by_scissor,
        "the table cuts as the scissor does"
    );
}
