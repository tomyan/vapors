//! A touch gesture's fling: the container the gesture scrolled coasts
//! at the lifted finger's velocity, within Chromium's bounds.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use vapors_dom::NodeId;
use vapors_elements::Document;
use vapors_platform::{Event, Instant, Platform, TouchPhase};

use super::DocumentInput;
use crate::scroll_fixture::{nested, offset};

fn touch(phase: TouchPhase, y: f32) -> Event {
    Event::Touch {
        id: 1,
        phase,
        x: 10.0,
        y,
    }
}

/// A finger down at `y` on the nested lists (scale 1) dragging 25 px
/// up, lifted, then the platform's fling at `vy` (the finger's way).
fn flick(y: f32, vy: f32) -> (Rc<RefCell<Document>>, NodeId, DocumentInput) {
    let (doc, outer, _) = nested();
    let (mut input, platform) = (DocumentInput::new(), Platform::detached());
    let events = [
        touch(TouchPhase::Started, y),
        touch(TouchPhase::Moved, y - 25.0),
        Event::Pan { dx: 0.0, dy: -25.0 },
        touch(TouchPhase::Ended, y - 25.0),
        Event::Fling { vx: 0.0, vy },
    ];
    for event in &events {
        input.give(&doc, event, &platform).expect("placed");
    }
    (doc, outer, input)
}

#[test]
fn a_flicked_list_coasts_on_after_the_finger_lifts() {
    // Given the outer list, dragged 25 px and flicked up at 600 px/s
    let (doc, outer, mut input) = flick(80.0, -600.0);
    assert!(input.flinging());

    // When
    input.advance_fling(
        &mut doc.borrow_mut(),
        Instant::now() + Duration::from_millis(100),
    );

    // Then
    assert!(offset(&doc, outer) > 30.0, "{}", offset(&doc, outer));
}

#[test]
fn a_finger_lifted_at_50_px_a_second_or_slower_does_not_fling() {
    // Given, When
    let (_, _, input) = flick(80.0, -50.0);

    // Then
    assert!(!input.flinging());
}

#[test]
fn a_fling_after_a_gesture_that_scrolled_nothing_starts_nothing() {
    // Given a flick, then a tap, which stops its fling
    let (doc, _, mut input) = flick(80.0, -600.0);
    let platform = Platform::detached();
    for event in [
        touch(TouchPhase::Started, 80.0),
        touch(TouchPhase::Ended, 80.0),
    ] {
        input.give(&doc, &event, &platform).expect("placed");
    }

    // When
    input
        .give(
            &doc,
            &Event::Fling {
                vx: 0.0,
                vy: -600.0,
            },
            &platform,
        )
        .expect("placed");

    // Then
    assert!(!input.flinging());
}

#[test]
fn a_fling_faster_than_16000_px_a_second_coasts_as_16000_does() {
    // Given three flicks, at and over the maximum
    let at = |vy: f32| {
        let (doc, outer, mut input) = flick(80.0, vy);
        let start = offset(&doc, outer);
        input.advance_fling(
            &mut doc.borrow_mut(),
            Instant::now() + Duration::from_millis(3),
        );
        offset(&doc, outer) - start
    };

    // When
    let (max, over, under) = (at(-16_000.0), at(-50_000.0), at(-15_000.0));

    // Then: Instant::now() differs by microseconds between flicks
    assert!((over - max).abs() < 1.0, "{over} vs {max}");
    assert!(max - under > 1.0, "{max} vs {under}");
}
