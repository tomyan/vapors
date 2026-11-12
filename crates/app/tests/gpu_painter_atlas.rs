//! GPU tier: a painter's glyph atlas is sized for its device unless
//! given a config, and a config the device cannot hold is refused,
//! naming the page and the device's side.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::{ElementsApp, NewPainterError, ScenePainter};
use vapors_elements::Document;
use vapors_gpu_harness::gpu;
use vapors_platform::{App, Frame, Instant, Next};
use vapors_renderer::clear;
use vapors_scene::Color;
use vapors_text::{AtlasConfig, FontSystem};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

#[test]
fn gpu_a_painter_given_an_atlas_over_the_device_side_refuses_naming_both() {
    // Given
    let gpu = gpu();
    let side = gpu.device.limits().max_texture_dimension_2d;

    // When
    let refused = ScenePainter::with_atlas(&gpu.device, FORMAT, AtlasConfig::fixed(side * 2)).err();

    // Then
    assert!(
        matches!(refused, Some(NewPainterError::Atlas(_))),
        "{refused:?}"
    );
    let message = refused.expect("refused").to_string();
    let (page, device) = (format!("{} px", side * 2), format!("{side} px"));
    assert!(
        message.contains(&page) && message.contains(&device),
        "{message}"
    );
}

#[test]
fn gpu_a_painter_given_an_atlas_the_device_holds_is_made() {
    // Given
    let gpu = gpu();

    // When
    let made = ScenePainter::with_atlas(&gpu.device, FORMAT, AtlasConfig::fixed(2048));

    // Then
    assert!(made.is_ok(), "{:?}", made.err());
}

#[test]
fn gpu_an_app_given_an_atlas_the_device_cannot_hold_stops_at_its_first_frame_naming_it() {
    // Given
    let gpu = gpu();
    let side = gpu.device.limits().max_texture_dimension_2d;
    let document = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    let mut app = ElementsApp::new(document).with_atlas(AtlasConfig::fixed(side * 2));
    let texture = clear(&gpu.device, &gpu.queue, 16, 16, Color::default());

    // When
    let next = app.frame(&Frame {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        texture: &texture,
        index: 0,
        scale_factor: 1.0,
        now: Instant::now(),
    });

    // Then
    assert!(matches!(next, Next::Exit), "{next:?}");
    let failure = app.failure().get().expect("a failure");
    assert!(failure.contains(&format!("{} px", side * 2)), "{failure}");
}
