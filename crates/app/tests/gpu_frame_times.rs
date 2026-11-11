//! GPU tier: an `ElementsApp` frame's phase times hold the frame's own
//! work (review-v75, M2): a per-frame hook runs before the phases, so
//! the time it takes is not in the Update phase, which 11-Q5's ceiling
//! will read.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use vapors_app::frame::{FrameTimes, Phase};
use vapors_app::{ElementsApp, FrameTimesHook, PerFrame};
use vapors_elements::Document;
use vapors_gpu_harness::{Gpu, gpu};
use vapors_platform::{App, Frame, Instant};
use vapors_renderer::clear;
use vapors_scene::Color;
use vapors_text::FontSystem;

/// Far longer than a warm empty document's update. A frame's update can
/// still pass it on a starved runner (482 ms for a cold first frame on
/// CI lavapipe), so the check takes the fastest of several frames.
const HOOK: Duration = Duration::from_millis(200);

/// Frames timed with the hook. A hook timed inside Update puts at least
/// `HOOK` in every one of them, so the fastest still fails the check.
const TIMED_FRAMES: u64 = 3;

#[test]
fn gpu_a_slow_per_frame_hook_is_not_in_the_update_phase() {
    // Given: a hook that sleeps once warmed up, and each frame's update time
    let document = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    let sleeping = Rc::new(Cell::new(false));
    let updates: Rc<RefCell<Vec<Duration>>> = Rc::default();
    let mut app = ElementsApp::new(document)
        .with_per_frame(Some(sleeping_hook(&sleeping)))
        .with_frame_times(update_recorder(&updates));
    let gpu = gpu();
    run_frame(&mut app, &gpu, 0);
    sleeping.set(true);

    // When
    for index in 1..=TIMED_FRAMES {
        run_frame(&mut app, &gpu, index);
    }

    // Then
    let updates = updates.borrow();
    let fastest = updates[1..].iter().min().expect("the timed frames ran");
    assert!(*fastest < HOOK, "update phases {updates:?}");
}

fn sleeping_hook(sleeping: &Rc<Cell<bool>>) -> PerFrame {
    let sleeping = Rc::clone(sleeping);
    Box::new(move |_| {
        if sleeping.get() {
            std::thread::sleep(HOOK);
        }
    })
}

fn update_recorder(updates: &Rc<RefCell<Vec<Duration>>>) -> FrameTimesHook {
    let into = Rc::clone(updates);
    Box::new(move |times: &FrameTimes| into.borrow_mut().push(times.of(Phase::Update)))
}

fn run_frame(app: &mut ElementsApp, gpu: &Gpu, index: u64) {
    let texture = clear(&gpu.device, &gpu.queue, 64, 64, Color::default());
    app.frame(&Frame {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        texture: &texture,
        index,
        scale_factor: 1.0,
        now: Instant::now(),
    });
}
