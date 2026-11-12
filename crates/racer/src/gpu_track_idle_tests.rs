//! GPU tier: the frame that pauses the race, or ends a paused camera
//! transition, is the last. It asks for no next frame and wakes no
//! loop, and the frames after it render nothing, as the browser
//! regressions' idle counts need (`scripts/check-racer-*.mjs`).
use crate::app::Racer;
use crate::gpu_frames::{idle_in_loop, looped, space};
use std::time::Duration;
use vapors_gpu_harness::{Gpu, gpu};
use vapors_platform::{App, Instant, Next, Platform};

/// A 60 Hz frame interval, as the browser's.
const INTERVAL: Duration = Duration::from_millis(16);

#[test]
fn gpu_the_frame_that_pauses_the_race_is_the_last() {
    // Given: racing in a loop
    let (gpu, platform) = (gpu(), Platform::detached());
    let mut racer = idle_in_loop(&gpu, &platform);
    racer.command("toggle").expect("drives");
    let mut now = Instant::now();
    for _ in 0..3 {
        now += INTERVAL;
        looped(&mut racer, &gpu, &platform, now);
    }

    // When: Space pauses; its event asks for the frame that shows it
    racer.event(space(true), &platform);
    assert!(
        platform.take_redraw_request(),
        "the pause asked for no frame"
    );
    now += INTERVAL;
    let before = racer.track.renders();
    let pausing = looped(&mut racer, &gpu, &platform, now);

    // Then: the paused scene is the last one rendered, so it renders
    // nothing (on the web a render polls its GPU error scope for frames)
    assert!(racer.run.paused);
    assert_eq!(pausing, (Next::Wait, false), "{}", why(&racer));
    assert_eq!(racer.track.renders(), before, "the pausing frame rendered");
    assert_nothing_follows(&mut racer, &gpu, &platform, now);
}

#[test]
fn gpu_the_frame_that_ends_a_paused_camera_transition_is_the_last() {
    // Given: idle in a loop; a camera toggle moves the camera while paused
    let (gpu, platform) = (gpu(), Platform::detached());
    let mut racer = idle_in_loop(&gpu, &platform);
    racer.command("camera").expect("toggles the camera");
    let mut now = Instant::now();

    // When: frames until the camera comes to rest (about 4 s)
    let mut settling = None;
    for _ in 0..1000 {
        now += INTERVAL;
        let before = racer.track.renders();
        let asked = looped(&mut racer, &gpu, &platform, now);
        if !racer.run.camera.moving {
            settling = Some((asked, racer.track.renders() - before));
            break;
        }
    }

    // Then: the frame at rest renders the camera's final pose and is the last
    let (asked, rendered) = settling.expect("the camera still moves after 1000 frames");
    assert_eq!(asked, (Next::Wait, false), "{}", why(&racer));
    assert_eq!(rendered, 1, "the frame at rest did not render the track");
    assert_nothing_follows(&mut racer, &gpu, &platform, now);
}

/// Three more frames, as a stray wake would draw: each asks for none,
/// wakes nothing and renders nothing.
fn assert_nothing_follows(racer: &mut Racer, gpu: &Gpu, platform: &Platform, mut now: Instant) {
    let before = racer.track.renders();
    for _ in 0..3 {
        now += INTERVAL;
        let asked = looped(racer, gpu, platform, now);
        assert_eq!(asked, (Next::Wait, false), "{}", why(racer));
    }
    assert_eq!(
        racer.track.renders(),
        before,
        "an idle frame rendered the track"
    );
}

/// Why the last frame asked for another, for a failing assertion.
fn why(racer: &Racer) -> String {
    format!(
        "last decision {:?}; surfaces {:?}",
        racer.app.last_decision(),
        racer.app.surface_states()
    )
}
