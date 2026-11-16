//! An event's time is the one the event loop read for it (#440): a
//! fling's `at` is the lift's time given to `translate`, not a clock
//! `Input` reads itself.

use std::time::Duration;

use super::*;
use winit::dpi::PhysicalPosition;
use winit::event::{DeviceId, Touch, TouchPhase};

fn touch(phase: TouchPhase, y: f64) -> WindowEvent {
    WindowEvent::Touch(Touch {
        device_id: DeviceId::dummy(),
        phase,
        location: PhysicalPosition::new(50.0, y),
        force: None,
        id: 0,
    })
}

#[test]
fn a_fling_is_timed_at_the_lift_time_the_event_loop_gave() {
    // Given a finger dragging up, each touch at a time a minute ahead of
    // the system clock
    let mut input = Input::new();
    let base = Instant::now() + Duration::from_secs(60);
    let at = |ms: u64| base + Duration::from_millis(ms);
    input.translate(&touch(TouchPhase::Started, 250.0), at(0));
    for n in 1..=10_u64 {
        let y = 250.0 - 8.0 * f64::from(u32::try_from(n).expect("small"));
        input.translate(&touch(TouchPhase::Moved, y), at(8 * n));
    }

    // When the finger lifts
    let lifted = input.translate(&touch(TouchPhase::Ended, 170.0), at(88));

    // Then
    let fling = lifted.iter().find_map(|event| match event {
        Event::Fling { at, .. } => Some(*at),
        _ => None,
    });
    assert_eq!(fling, Some(at(88)), "{lifted:?}");
}
