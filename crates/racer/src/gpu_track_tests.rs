//! GPU tier: the world drawn through the HUD's `gpu-surface`. It renders
//! every frame while the car moves and the app asks for the next; a race
//! standing still asks for frames and renders nothing; paused, the app
//! waits and nothing renders; resuming animates it again; the HUD's
//! controls draw over it; a press on a `HoldButton` drives the car.
use crate::gpu_frames::{
    drive, frame, frames, hold_button, idle, pixel, press, settle, space, world_alone,
};
use vapors_gpu_harness::gpu;
use vapors_platform::{App, Next, Platform};

#[test]
fn gpu_the_world_renders_on_every_frame_while_the_car_moves() {
    // Given
    let gpu = gpu();
    let mut racer = idle(&gpu);
    let before = racer.track.renders();

    // When
    drive(&mut racer);
    let racing = frames(&mut racer, &gpu, 4);

    // Then: all but the first, which comes 0 s after the start, so the
    // car has not moved yet
    assert_eq!(racing, [Next::Continue; 4]);
    assert_eq!(racer.track.renders() - before, 3);
}

#[test]
fn gpu_a_race_standing_still_asks_for_frames_and_renders_nothing() {
    // Given
    let gpu = gpu();
    let mut racer = idle(&gpu);
    let before = racer.track.renders();

    // When: racing, no pedal held, the fixed camera at rest
    racer.command("toggle").expect("drives");
    let racing = frames(&mut racer, &gpu, 3);

    // Then: the clock runs, so frames continue; the world is unchanged
    assert_eq!(racing, [Next::Continue; 3]);
    assert_eq!(racer.track.renders(), before);
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
    drive(&mut racer);
    frames(&mut racer, &gpu, 2);
    racer.command("toggle").expect("pauses");
    settle(&mut racer, &gpu);
    let before = racer.track.renders();

    // When
    drive(&mut racer);
    let resumed = frames(&mut racer, &gpu, 3);

    // Then: all but the first, 0 s after resuming
    assert_eq!(resumed, [Next::Continue; 3]);
    assert_eq!(racer.track.renders() - before, 2);
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
    let track = world_alone(&racer, &gpu);
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
