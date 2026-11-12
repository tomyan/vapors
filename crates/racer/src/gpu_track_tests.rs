//! GPU tier: the track drawn through the HUD's `gpu-surface`. It renders
//! every frame while racing and the app asks for the next; paused, the
//! app waits and nothing renders; resuming animates it again; the HUD's
//! controls draw over it; a press on a `HoldButton` drives the car.
use crate::gpu_frames::{
    frame, frames, hold_button, idle, pixel, press, settle, space, track_alone,
};
use vapors_gpu_harness::gpu;
use vapors_platform::{App, Next, Platform};

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
    let after_space = frames(&mut racer, &gpu, 2);
    let paused = racer.track.renders();
    let idle_frames = frames(&mut racer, &gpu, 3);

    // Then
    assert!(racer.run.paused);
    assert!(after_space.contains(&Next::Wait), "{after_space:?}");
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
    // background (#b9d7bc) over the track; away from the controls is
    // the track as it renders alone
    let track = track_alone(&racer, &gpu);
    let (inside, open) = ([x + 8.0, y + 8.0], [x - width * 4.0, y - height * 4.0]);
    assert_eq!(pixel(&rgba, inside), [185, 215, 188, 255]);
    assert_ne!(
        pixel(&track, inside),
        [185, 215, 188, 255],
        "the track shows the button's colour"
    );
    assert_ne!(
        pixel(&track, open),
        [255, 0, 255, 255],
        "the track drew nothing there"
    );
    assert_eq!(pixel(&rgba, open), pixel(&track, open));
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
