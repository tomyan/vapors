//! GPU tier: an `ElementsApp` at 4 samples through its own frame path
//! (a new view of the surface's texture each frame) makes one 4x
//! texture, on its first frame; and its samples are refused once its
//! painter is made.

use vapors_dom::Dom;
use vapors_platform::{App, Frame, Next};
use vapors_renderer::Samples;

use crate::support_surface::start_rig::{rig, start, start_at};
use crate::support_surface::{document, update};

/// Frames of an app at 4 samples into one 64-pixel surface texture, a
/// class toggled before each so every frame paints: each frame's 4x
/// textures made (`FrameCounts::multisampled_made`).
///
/// # Panics
///
/// When a frame fails: its counts would be stale.
fn made_per_frame(frames: u64) -> Vec<u64> {
    let gpu = vapors_gpu_harness::gpu();
    let css = ".on { background-color: #f00; } div { height: 8px; }";
    let mut doc = document(css, &[(None, "div", "")]);
    update(&mut doc, 1.0);
    let div = doc.children(doc.body())[0];
    let shared = std::rc::Rc::new(std::cell::RefCell::new(doc));
    let mut app =
        vapors_app::ElementsApp::new(std::rc::Rc::clone(&shared)).with_samples(Samples::Four);
    let size = vapors_platform::Start {
        width: 64,
        height: 64,
        ..start_at(&gpu)
    };
    app.start(&size).expect("starts");
    let colour = vapors_scene::Color::default();
    let texture = vapors_renderer::clear(&gpu.device, &gpu.queue, 64, 64, colour);
    let now = vapors_platform::Instant::now();
    (0..frames)
        .map(|index| {
            let class = if index % 2 == 0 { "on" } else { "" };
            shared.borrow_mut().set_attribute(div, "class", class);
            let frame = Frame {
                adapter: &gpu.adapter,
                device: &gpu.device,
                queue: &gpu.queue,
                texture: &texture,
                index,
                scale_factor: 1.0,
                now,
                resumed: false,
            };
            let next = app.frame(&frame);
            assert_ne!(
                next,
                Next::Exit,
                "frame {index} failed: {:?}",
                app.failure().get()
            );
            app.frame_counts().multisampled_made
        })
        .collect()
}

#[test]
fn gpu_an_app_at_4_samples_makes_one_4x_texture_over_its_frames_at_one_size() {
    // When: four frames, the app making a new view of the surface's
    // texture each (the fault shows on the second; 600 frames, as first
    // measured, cost about 0.5 s more and pin nothing more)
    let made = made_per_frame(4);

    // Then: one, on the first frame (16 bytes a device pixel: 41.5 MB a
    // frame at the Pixel 9 Pro's 1080 x 2400 when it was made every
    // frame, 2026-11-17)
    assert_eq!(made, [1, 0, 0, 0]);
}

#[test]
#[should_panic(expected = "set_samples after the painter was made")]
fn gpu_samples_set_after_the_app_started_are_refused() {
    let (mut app, _, gpu) = rig();
    start(&mut app, &gpu).expect("starts");
    app.set_samples(Samples::Four);
}
