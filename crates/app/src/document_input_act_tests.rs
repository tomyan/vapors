use std::time::Duration;

use vapors_platform::{Event, Instant, Platform, TouchPhase};

use super::DocumentInput;
use crate::scroll_fixture::nested;
use crate::thumb_fade::{FADE_DELAY, Phase};

#[test]
fn a_touch_times_the_thumbs_fade_by_the_platforms_clock_not_the_systems() {
    // Given a platform a minute ahead of the system clock (#440)
    let (doc, _, _) = nested();
    let (mut input, platform) = (DocumentInput::new(), Platform::detached());
    let then = Instant::now() + Duration::from_secs(60);
    platform.set_now(Some(then));
    let down = Event::Touch {
        id: 1,
        phase: TouchPhase::Started,
        x: 10.0,
        y: 80.0,
    };

    // When the first finger lands, after the mouse
    input.give(&doc, &down, &platform).expect("placed");

    // Then the floor's fade counts from the platform's time
    assert_eq!(
        input.touch.thumb_phase(then),
        Phase::Shown(then + FADE_DELAY)
    );
}

#[test]
fn an_event_stamped_before_the_last_frame_is_handled_at_the_frame_and_counted() {
    // Given a frame a second after the event's stamp (#440, review L1)
    let (doc, _, _) = nested();
    let (mut input, platform) = (DocumentInput::new(), Platform::detached());
    let stamped = Instant::now() + Duration::from_secs(60);
    let frame = stamped + Duration::from_secs(1);
    input.advance_fling(&mut doc.borrow_mut(), frame);
    platform.set_now(Some(stamped));
    let down = Event::Touch {
        id: 1,
        phase: TouchPhase::Started,
        x: 10.0,
        y: 80.0,
    };

    // When
    input.give(&doc, &down, &platform).expect("placed");

    // Then: the floor's fade counts from the frame, and the clamp is counted
    assert_eq!(
        input.touch.thumb_phase(frame),
        Phase::Shown(frame + FADE_DELAY)
    );
    assert_eq!(input.event_times_clamped(), 1);
}
