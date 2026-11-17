//! GPU tier: the painter's 4x textures are one for each viewport size a
//! frame draws at, the same texture while each frame draws at the size
//! whatever view it is given, a new one at a new size, the old dropped,
//! and none at 1 sample; a target naming its own multisampled view is
//! refused.

use vapors_elements::{Document, Viewport};
use vapors_gpu_harness::{Gpu, gpu};
use vapors_renderer::{FrameEncoder, PassFormat, Samples, Target, clear};
use vapors_scene::{Color, Size};
use vapors_text::FontSystem;

use crate::ScenePainter;

/// A painter at `samples` on `gpu`.
fn painter(gpu: &Gpu, samples: Samples) -> ScenePainter {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    ScenePainter::new(&gpu.device, PassFormat { format, samples }).expect("Rgba8Unorm")
}

/// A target view `side` pixels square on `gpu`.
fn view_of(gpu: &Gpu, side: u32) -> wgpu::TextureView {
    let texture = clear(&gpu.device, &gpu.queue, side, side, Color::default());
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// `doc`, an empty page, drawn by `painter` on `gpu` into `view`, `side`
/// pixels square, the target naming `multisampled` when given.
fn draw_into(
    (gpu, painter, doc): (&Gpu, &mut ScenePainter, &mut Document),
    (view, side): (&wgpu::TextureView, u32),
    multisampled: Option<&wgpu::TextureView>,
) {
    let css_px = f32::from(u16::try_from(side).expect("small"));
    let viewport = Viewport {
        width: css_px,
        height: css_px,
        scale: 1.0,
    };
    let updated = doc.update(viewport).expect("updates");
    let encoder = FrameEncoder::new(&gpu.device);
    let target = Target {
        device: &gpu.device,
        queue: &gpu.queue,
        encoder: &encoder,
        view,
        multisampled,
        viewport: Size {
            width: side,
            height: side,
        },
        scissor: None,
    };
    painter.draw(&target, updated).expect("paints");
    encoder.submit(&gpu.queue).expect("no refusal");
}

/// An empty document.
fn page() -> Document {
    Document::new(FontSystem::bundle_only())
}

#[test]
fn gpu_a_4_sample_painter_keeps_one_texture_for_frames_into_new_views_of_one_size() {
    // Given
    let gpu = gpu();
    let (mut painter, mut doc) = (painter(&gpu, Samples::Four), page());

    // When: two frames, each into a new view of a 32 square, as an app's
    // surface gives
    draw_into(
        (&gpu, &mut painter, &mut doc),
        (&view_of(&gpu, 32), 32),
        None,
    );
    let first = painter.multisampled.held_at((32, 32)).expect("held");
    draw_into(
        (&gpu, &mut painter, &mut doc),
        (&view_of(&gpu, 32), 32),
        None,
    );

    // Then: the same texture, the only one
    assert_eq!(painter.multisampled.held_at((32, 32)), Some(first));
    assert_eq!(painter.multisampled.held(), 1);
}

#[test]
fn gpu_a_frame_at_a_new_size_makes_a_texture_and_drops_the_old() {
    // Given
    let gpu = gpu();
    let (mut painter, mut doc) = (painter(&gpu, Samples::Four), page());

    // When
    draw_into(
        (&gpu, &mut painter, &mut doc),
        (&view_of(&gpu, 32), 32),
        None,
    );
    draw_into(
        (&gpu, &mut painter, &mut doc),
        (&view_of(&gpu, 16), 16),
        None,
    );

    // Then
    assert!(painter.multisampled.held_at((32, 32)).is_none());
    assert!(painter.multisampled.held_at((16, 16)).is_some());
    assert_eq!(painter.multisampled.held(), 1);
}

#[test]
fn gpu_a_1_sample_painter_holds_no_texture() {
    let gpu = gpu();
    let mut painter = painter(&gpu, Samples::One);
    draw_into(
        (&gpu, &mut painter, &mut page()),
        (&view_of(&gpu, 32), 32),
        None,
    );
    assert_eq!(painter.multisampled.held(), 0);
}

#[test]
#[should_panic(expected = "a painter was given a target with a multisampled view")]
fn gpu_a_target_naming_its_own_multisampled_view_is_refused() {
    let gpu = gpu();
    let (view, own) = (view_of(&gpu, 8), view_of(&gpu, 8));
    let mut painter = painter(&gpu, Samples::One);
    draw_into((&gpu, &mut painter, &mut page()), (&view, 8), Some(&own));
}

#[test]
fn gpu_a_painter_at_4_samples_on_a_format_not_guaranteed_them_is_refused_naming_it() {
    // When: a format the painter refuses anyway, at 4 samples, which is
    // checked first
    let gpu = gpu();
    let pass = PassFormat {
        format: wgpu::TextureFormat::Rgba32Float,
        samples: Samples::Four,
    };
    let refused = ScenePainter::new(&gpu.device, pass).map(|_| ());

    // Then
    assert!(
        matches!(refused, Err(crate::NewPainterError::Samples(at)) if at == pass),
        "{refused:?}"
    );
}
