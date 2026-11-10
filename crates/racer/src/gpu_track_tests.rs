//! GPU tier: the track drawn through the HUD's `gpu-surface`. It renders
//! every frame while racing and the app asks for the next; paused, the
//! app waits and nothing renders; resuming animates it again; the HUD's
//! controls draw over it; a press on a `HoldButton` drives the car.
use crate::app::Racer;
use std::num::NonZeroU32;
use vapors_gpu_harness::{Gpu, gpu, read_rgba8};
use vapors_platform::{App, Event, Frame, Instant, Modifiers, Next, Platform};
use vapors_renderer::clear;
use vapors_scene::Color;

const WIDTH: u32 = 1100;
const HEIGHT: u32 = 720;
/// What the frame's texture holds before the app draws.
const MAGENTA: Color = Color {
    r: 1.0,
    g: 0.0,
    b: 1.0,
    a: 1.0,
};

/// One racer frame into a fresh texture: what it asked for next, and the pixels.
fn frame(racer: &mut Racer, gpu: &Gpu) -> (Next, Vec<u8>) {
    let texture = clear(&gpu.device, &gpu.queue, WIDTH, HEIGHT, MAGENTA);
    let next = racer.frame(&Frame {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        texture: &texture,
        index: 1,
        scale_factor: 1.0,
        now: Instant::now(),
    });
    assert_eq!(racer.app.failure().get(), None, "the frame failed");
    (next, read_rgba8(&gpu.device, &gpu.queue, &texture))
}

/// `count` frames: what each asked for next.
fn frames(racer: &mut Racer, gpu: &Gpu, count: usize) -> Vec<Next> {
    (0..count).map(|_| frame(racer, gpu).0).collect()
}

/// Frames until one asks for none; refused after 10, which a paused
/// racer never needs (it settles in 3 or fewer).
fn settle(racer: &mut Racer, gpu: &Gpu) {
    let settled = (0..10).any(|_| frame(racer, gpu).0 == Next::Wait);
    assert!(
        settled,
        "the racer still asks for frames after 10 while paused"
    );
}

/// A racer drawn, then paused and idle: its frames ask for none.
fn idle(gpu: &Gpu) -> Racer {
    let mut racer = Racer::new();
    settle(&mut racer, gpu);
    racer
}

fn space(pressed: bool) -> Event {
    Event::Key {
        key: " ".into(),
        code: "Space".into(),
        pressed,
        repeat: false,
        modifiers: Modifiers::default(),
    }
}

/// The CSS rectangle of the `HoldButton` named `name`: x, y, width, height.
fn hold_button(racer: &Racer, name: &str) -> [f32; 4] {
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

fn press(racer: &mut Racer, [x, y]: [f32; 2], pressed: bool) {
    let platform = Platform::detached();
    racer.event(Event::PointerMoved { x, y }, &platform);
    let button = Event::PrimaryButton {
        pressed,
        clicks: NonZeroU32::MIN,
        modifiers: Modifiers::default(),
    };
    racer.event(button, &platform);
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // a pixel on the frame
fn pixel(rgba: &[u8], [x, y]: [f32; 2]) -> [u8; 4] {
    let at = (y as usize * WIDTH as usize + x as usize) * 4;
    rgba[at..at + 4].try_into().expect("four bytes")
}

#[test]
fn gpu_the_track_renders_on_every_frame_while_racing() {
    // Given
    let gpu = gpu();
    let mut racer = idle(&gpu);
    let before = racer.track.renders();

    // When
    racer.command("toggle").expect("drives");
    let racing = frames(&mut racer, &gpu, 4);

    // Then
    assert_eq!(racing, [Next::Continue; 4]);
    assert_eq!(racer.track.renders() - before, 4);
}

#[test]
fn gpu_pausing_stops_the_animation_and_the_app_waits_with_nothing_rendered() {
    // Given
    let gpu = gpu();
    let mut racer = idle(&gpu);
    racer.command("toggle").expect("drives");
    frames(&mut racer, &gpu, 2);

    // When: Space with nothing focused is the racer's
    racer.event(space(true), &Platform::detached());
    settle(&mut racer, &gpu);
    let paused = racer.track.renders();
    let idle_frames = frames(&mut racer, &gpu, 3);

    // Then
    assert!(racer.run.paused);
    assert_eq!(idle_frames, [Next::Wait; 3]);
    assert_eq!(racer.track.renders(), paused);
}

#[test]
fn gpu_resuming_animates_the_track_again() {
    // Given: raced, paused, idle
    let gpu = gpu();
    let mut racer = idle(&gpu);
    racer.command("toggle").expect("drives");
    frames(&mut racer, &gpu, 2);
    racer.command("toggle").expect("pauses");
    settle(&mut racer, &gpu);
    let before = racer.track.renders();

    // When
    racer.command("toggle").expect("drives");
    let resumed = frames(&mut racer, &gpu, 3);

    // Then
    assert_eq!(resumed, [Next::Continue; 3]);
    assert_eq!(racer.track.renders() - before, 3);
}

#[test]
fn gpu_the_hud_draws_over_the_track() {
    // Given
    let gpu = gpu();
    let mut racer = idle(&gpu);
    let [x, y, width, height] = hold_button(&racer, "gas");

    // When
    let (_, rgba) = frame(&mut racer, &gpu);

    // Then: inside the gas button's border, clear of its label, is its
    // background (#b9d7bc); the middle of the window is the track
    assert_eq!(pixel(&rgba, [x + 8.0, y + 8.0]), [185, 215, 188, 255]);
    let middle = pixel(&rgba, [x - width * 4.0, y - height * 4.0]);
    assert_ne!(middle, [255, 0, 255, 255], "the track was not drawn");
    assert_eq!(middle[3], 255);
}

#[test]
fn gpu_a_press_on_the_gas_button_drives_the_car() {
    // Given
    let gpu = gpu();
    let mut racer = idle(&gpu);
    let [x, y, width, height] = hold_button(&racer, "gas");

    // When
    press(&mut racer, [x + width / 2.0, y + height / 2.0], true);
    frames(&mut racer, &gpu, 5);

    // Then
    assert!(!racer.run.paused, "a held pedal starts the race");
    assert!(racer.hud.gas.get());
    assert!(racer.run.race.speed > 0.0, "{}", racer.run.race.speed);
}
