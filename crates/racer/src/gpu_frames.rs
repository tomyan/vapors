//! What the racer's GPU tests share: a frame into a fresh texture, the
//! racer settled and idle, and the HUD's buttons and pixels.
use crate::app::Racer;
use std::num::NonZeroU32;
use vapors_app::SurfaceSize;
use vapors_gpu_harness::{Gpu, read_rgba8};
use vapors_platform::{App, Event, Frame, Instant, Modifiers, Next, Platform};
use vapors_renderer::clear;
use vapors_scene::Color;

/// The world's draws when it renders: the ground and the nine parts
/// (`world_geometry.rs`'s `Part::ALL`), the one answer the racer's GPU
/// tests count from.
pub(crate) const WORLD_DRAWS: u64 = 1 + crate::world_geometry::Part::ALL.len() as u64;

pub(crate) const WIDTH: u32 = 1100;
pub(crate) const HEIGHT: u32 = 720;
/// What the frame's texture holds before the app draws.
pub(crate) const MAGENTA: Color = Color {
    r: 1.0,
    g: 0.0,
    b: 1.0,
    a: 1.0,
};

/// One racer frame into a fresh texture: what it asked for next, and the pixels.
pub(crate) fn frame(racer: &mut Racer, gpu: &Gpu) -> (Next, Vec<u8>) {
    let drawn = frame_at(racer, gpu, Instant::now());
    assert_eq!(racer.app.failure().get(), None, "the frame failed");
    drawn
}

/// The window a frame draws into: its device size and scale.
pub(crate) const WINDOW: SurfaceSize = SurfaceSize {
    width: WIDTH,
    height: HEIGHT,
    scale: 1.0,
};

/// One racer frame at `now`, whether or not it failed.
pub(crate) fn frame_at(racer: &mut Racer, gpu: &Gpu, now: Instant) -> (Next, Vec<u8>) {
    frame_in(racer, gpu, WINDOW, now)
}

/// One racer frame at `now` into a fresh `window`-sized texture, whether
/// or not it failed.
pub(crate) fn frame_in(
    racer: &mut Racer,
    gpu: &Gpu,
    window: SurfaceSize,
    now: Instant,
) -> (Next, Vec<u8>) {
    let SurfaceSize { width, height, .. } = window;
    let texture = clear(&gpu.device, &gpu.queue, width, height, MAGENTA);
    let next = racer.frame(&Frame {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        texture: &texture,
        index: 1,
        scale_factor: f64::from(window.scale),
        now,
    });
    (next, read_rgba8(&gpu.device, &gpu.queue, &texture))
}

/// One frame at `now` as the window's loop runs it, `drawn` after:
/// what it asked for next, and whether a handle woke the loop during
/// it, which draws another frame too.
pub(crate) fn looped(
    racer: &mut Racer,
    gpu: &Gpu,
    platform: &Platform,
    now: Instant,
) -> (Next, bool) {
    let (next, _) = frame_at(racer, gpu, now);
    assert_eq!(racer.app.failure().get(), None, "the frame failed");
    racer.drawn(platform);
    (next, platform.take_redraw_request())
}

/// An idle racer whose app keeps `platform`'s waker, as after the
/// loop's first frame: a handle call wakes `platform` from then on.
pub(crate) fn idle_in_loop(gpu: &Gpu, platform: &Platform) -> Racer {
    idle_in_loop_at(gpu, platform, vapors_renderer::Samples::Four)
}

/// As [`idle_in_loop`], the racer drawn at `samples`.
pub(crate) fn idle_in_loop_at(
    gpu: &Gpu,
    platform: &Platform,
    samples: vapors_renderer::Samples,
) -> Racer {
    let mut racer = idle_at(gpu, samples);
    racer.drawn(platform);
    assert!(
        !platform.take_redraw_request(),
        "an idle racer woke the loop"
    );
    racer
}

/// `count` frames: what each asked for next.
pub(crate) fn frames(racer: &mut Racer, gpu: &Gpu, count: usize) -> Vec<Next> {
    (0..count).map(|_| frame(racer, gpu).0).collect()
}

/// Frames until one asks for none; refused after 10, which a paused
/// racer never needs (it settles in 3 or fewer).
pub(crate) fn settle(racer: &mut Racer, gpu: &Gpu) {
    let settled = (0..10).any(|_| frame(racer, gpu).0 == Next::Wait);
    assert!(
        settled,
        "the racer still asks for frames after 10 while paused"
    );
}

/// A racer drawn, then paused and idle: its frames ask for none.
pub(crate) fn idle(gpu: &Gpu) -> Racer {
    idle_at(gpu, vapors_renderer::Samples::Four)
}

/// As [`idle`], the racer drawn at `samples`: 1 for a test of when frames
/// come and render, which the sample count does not change, that draws
/// tens of frames (a camera transition, about 70): at 4 samples one took
/// 13.5 s on llvmpipe (2026-11-17).
pub(crate) fn idle_at(gpu: &Gpu, samples: vapors_renderer::Samples) -> Racer {
    let mut racer = Racer::with_samples(samples);
    settle(&mut racer, gpu);
    racer
}

/// Starts the race with the gas held, so the car moves and the world
/// changes on every frame: a race standing still, its camera settled,
/// changes nothing and renders nothing.
pub(crate) fn drive(racer: &mut Racer) {
    if racer.run.paused {
        racer.command("toggle").expect("drives");
    }
    racer.run.driving.key("KeyW", true);
}

pub(crate) fn space(pressed: bool) -> Event {
    Event::Key {
        key: " ".into(),
        code: "Space".into(),
        pressed,
        repeat: false,
        modifiers: Modifiers::default(),
    }
}

/// The CSS rectangle of the `HoldButton` named `name`: x, y, width, height.
pub(crate) fn hold_button(racer: &Racer, name: &str) -> [f32; 4] {
    let doc = racer.document.borrow();
    let elements = doc.elements();
    let mut nodes = elements.iter().copied();
    let node = nodes.find(|n| doc.attribute(*n, "data-hold") == Some(name));
    let node = node.expect("the HUD has the button");
    let rect = doc
        .painted()
        .expect("drawn")
        .border_box(node)
        .expect("laid out");
    [rect.x, rect.y, rect.width, rect.height]
}

pub(crate) fn press(racer: &mut Racer, [x, y]: [f32; 2], pressed: bool) {
    let platform = Platform::detached();
    racer.event(Event::PointerMoved { x, y }, &platform);
    let button = Event::PrimaryButton {
        pressed,
        clicks: NonZeroU32::MIN,
        modifiers: Modifiers::default(),
    };
    racer.event(button, &platform);
}

pub(crate) fn pixel(rgba: &[u8], at: [f32; 2]) -> [u8; 4] {
    pixel_in(rgba, WIDTH, at)
}

/// The pixel at device `[x, y]` of a frame `width` device pixels wide.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // a pixel on the frame
pub(crate) fn pixel_in(rgba: &[u8], width: u32, [x, y]: [f32; 2]) -> [u8; 4] {
    let at = (y as usize * width as usize + x as usize) * 4;
    rgba[at..at + 4].try_into().expect("four bytes")
}

/// The world alone, as the racer shows it now, drawn as the racer
/// draws it (at 4 samples, `Racer::new`) by a fresh `TrackWorld` at the
/// window's size: what the HUD's frame shows where no control covers it.
pub(crate) fn world_alone(racer: &Racer, gpu: &Gpu) -> Vec<u8> {
    world_alone_in(racer, gpu, WINDOW)
}

/// As [`world_alone`], in a window of device size `size`.
pub(crate) fn world_alone_in(racer: &Racer, gpu: &Gpu, size: SurfaceSize) -> Vec<u8> {
    let (race, camera) = (&racer.run.race, &racer.run.camera);
    let mut rig =
        crate::gpu_world_rig::Rig::window(race, camera, size, vapors_renderer::Samples::Four);
    rig.frame(gpu).1
}
