//! The painter's clip entries (`painter_clips.rs`): an entry follows its
//! frame's transform entry when that moves, and a held clip naming a
//! frame entry given back is refused (the release order, item 4 of
//! `docs/plans/flat-scene-f3.md`).

use vapors_dom::Dom;
use vapors_elements::{BatchFrame, Document, Viewport};
use vapors_gpu_harness::gpu;
use vapors_renderer::{FrameEncoder, Target};
use vapors_scene::{Rect, Size};
use vapors_text::FontSystem;

use super::{Clips, assert_whole};

/// The frame of the one batch of a page of one red div.
fn a_frame() -> BatchFrame {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet("app.css", "div { height: 2px; background-color: #f00; }")
        .expect("parses");
    let div = doc.create_element("div");
    doc.append(doc.body(), div);
    let view = Viewport {
        width: 10.0,
        height: 10.0,
        scale: 1.0,
    };
    let painted = doc.update(view).expect("updates");
    painted.kept_batches(Rect::new(0.0, 0.0, 10.0, 10.0)).frames[0]
}

/// `body` run with a target on a fresh 8 px square of `gpu` and a new
/// encoder, its writes submitted.
fn with_target(
    gpu: &vapors_gpu_harness::Gpu,
    clips: &mut Clips,
    body: impl FnOnce(&mut Clips, &Target<'_>),
) {
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("clips test"),
        size: wgpu::Extent3d {
            width: 8,
            height: 8,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
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
            width: 8,
            height: 8,
        },
        scissor: None,
    };
    body(clips, &target);
    encoder.submit(&gpu.queue).expect("no refusal");
}

#[test]
fn gpu_clip_entry_follows_its_frame_entry_after_the_frame_leaves_and_returns() {
    // Given: a clip drawn in a frame whose transform entry is 5
    let (frame, clip) = (a_frame(), Rect::new(0.0, 0.0, 4.0, 4.0));
    let gpu = gpu();
    let mut clips = Clips::new(&gpu.device);
    let mut first = 0;
    with_target(&gpu, &mut clips, |clips, at| {
        clips.begin_frame();
        first = clips.entry(at, (frame, 5), (clip.into(), 0)).expect("room");
    });

    // When: drawn again, its frame's entry now 9
    let mut again = 0;
    with_target(&gpu, &mut clips, |clips, at| {
        clips.begin_frame();
        again = clips.entry(at, (frame, 9), (clip.into(), 0)).expect("room");
    });

    // Then: the same entry, naming the frame's new one
    assert_eq!(again, first);
    assert_eq!(clips.table.entry(again).frame, 9);
}

#[test]
#[should_panic(expected = "given back while the clip is held")]
fn gpu_a_frame_entry_given_back_under_a_held_clip_is_refused() {
    // Given: a clip held, naming frame entry 5
    let gpu = gpu();
    let mut clips = Clips::new(&gpu.device);
    with_target(&gpu, &mut clips, |clips, at| {
        clips.begin_frame();
        clips.entry(
            at,
            (a_frame(), 5),
            (Rect::new(0.0, 0.0, 4.0, 4.0).into(), 0),
        );
    });

    // When: frame entry 5 is given back
    clips.assert_none_names(&[5]);
}

#[test]
#[should_panic(expected = "given back while it is held")]
fn gpu_a_frames_entry_given_back_under_a_held_item_entry_is_refused() {
    // Given: a frame's entry and an item entry under it, drawn
    let gpu = gpu();
    let mut clips = Clips::new(&gpu.device);
    let rect = Rect::new(0.0, 0.0, 4.0, 4.0);
    let mut parent = 0;
    with_target(&gpu, &mut clips, |clips, at| {
        clips.begin_frame();
        parent = clips.frame_clip(at, (a_frame(), rect)).expect("room");
        clips.entry(at, (a_frame(), 5), (rect.into(), parent));
    });

    // When: a frame draws the item entry alone
    with_target(&gpu, &mut clips, |clips, at| {
        clips.begin_frame();
        clips.entry(at, (a_frame(), 5), (rect.into(), parent));
    });
    clips.release_undrawn(&gpu.queue);
}

#[test]
#[should_panic(expected = "given back while it is held")]
fn gpu_a_frames_square_entry_given_back_under_its_held_rounded_one_is_refused() {
    // Given: a frame's square entry and its rounded one, drawn
    let gpu = gpu();
    let mut clips = Clips::new(&gpu.device);
    let rect = Rect::new(0.0, 0.0, 4.0, 4.0);
    let round = vapors_scene::Clip::rounded(rect, [1.0; 4]);
    let mut square = 0;
    with_target(&gpu, &mut clips, |clips, at| {
        clips.begin_frame();
        square = clips.frame_clip(at, (a_frame(), rect)).expect("room");
        clips.frame_round(at, (a_frame(), round, square));
    });

    // When: a frame draws the rounded entry alone
    with_target(&gpu, &mut clips, |clips, at| {
        clips.begin_frame();
        clips.frame_round(at, (a_frame(), round, square));
    });
    clips.release_undrawn(&gpu.queue);
}

#[test]
fn a_clip_with_whole_device_pixel_edges_is_accepted() {
    assert_whole(Rect::new(-3.0, 0.0, 16.0, 11.0));
}

#[test]
#[should_panic(
    expected = "Rect { left: 0.0, top: 0.0, right: 16.5, bottom: 11.0 } has an edge \
                           that is not a whole device pixel"
)]
fn a_clip_with_an_edge_half_a_device_pixel_off_is_refused() {
    assert_whole(Rect::new(0.0, 0.0, 16.5, 11.0));
}

#[test]
fn gpu_a_rounded_clip_takes_its_own_entry_holding_its_radii() {
    // Given: one rectangle, square and with rounded corners
    let rect = Rect::new(0.0, 0.0, 4.0, 4.0);
    let rounded = vapors_scene::Clip::rounded(rect, [2.0; 4]);
    let (gpu, frame) = (gpu(), a_frame());
    let mut clips = Clips::new(&gpu.device);
    let (mut square_at, mut rounded_at) = (0, 0);

    // When
    with_target(&gpu, &mut clips, |clips, at| {
        clips.begin_frame();
        square_at = clips.entry(at, (frame, 5), (rect.into(), 0)).expect("room");
        rounded_at = clips.entry(at, (frame, 5), (rounded, 0)).expect("room");
    });

    // Then: two entries, the rounded one with its radii
    assert_ne!(square_at, rounded_at);
    assert_eq!(clips.table.entry(square_at).radii, [0.0; 4]);
    assert_eq!(clips.table.entry(rounded_at).radii, [2.0; 4]);
}

#[test]
#[should_panic(expected = "reached the clip table cut to an outer rectangle")]
fn gpu_a_clip_cut_to_an_outer_rectangle_is_refused() {
    // Given: a rounded clip cut on the page, as only the flat page view cuts one
    let rounded = vapors_scene::Clip::rounded(Rect::new(0.0, 0.0, 8.0, 8.0), [2.0; 4]);
    let cut = rounded.cut(Rect::new(0.0, 0.0, 4.0, 4.0));
    let gpu = gpu();
    let mut clips = Clips::new(&gpu.device);

    // When, Then
    with_target(&gpu, &mut clips, |clips, at| {
        clips.begin_frame();
        clips.entry(at, (a_frame(), 5), (cut, 0));
    });
}
