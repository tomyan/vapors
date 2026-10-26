//! GPU tier: an `ElementsApp` frame's phase times hold the frame's own
//! work (review-v75, M2): a per-frame hook runs before the phases, so
//! the time it takes is not in the Update phase, which 11-Q5's ceiling
//! will read.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use vapors_app::ElementsApp;
use vapors_app::frame::{FrameTimes, Phase};
use vapors_elements::Document;
use vapors_gpu_harness::gpu;
use vapors_platform::{App, Frame, Instant};
use vapors_renderer::clear;
use vapors_scene::Color;
use vapors_text::FontSystem;

/// Far longer than an empty document's update, even at a high load.
const HOOK: Duration = Duration::from_millis(200);

#[test]
fn gpu_a_slow_per_frame_hook_is_not_in_the_update_phase() {
    // Given: a hook that sleeps, and the frame's times kept
    let document = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    let kept: Rc<RefCell<Option<FrameTimes>>> = Rc::default();
    let into = Rc::clone(&kept);
    let mut app = ElementsApp::new(document)
        .with_per_frame(Some(Box::new(|_| std::thread::sleep(HOOK))))
        .with_frame_times(Box::new(move |times| *into.borrow_mut() = Some(*times)));
    let gpu = gpu();
    let texture = clear(&gpu.device, &gpu.queue, 64, 64, Color::default());

    // When
    app.frame(&Frame {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        texture: &texture,
        index: 0,
        scale_factor: 1.0,
        now: Instant::now(),
    });

    // Then
    let times = kept.borrow().expect("the frame ran");
    assert!(times.of(Phase::Update) < HOOK, "{times:?}");
}
