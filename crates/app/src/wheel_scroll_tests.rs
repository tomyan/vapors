use std::time::Duration;

use vapors_platform::Instant;

use super::*;
use crate::scroll_fixture::{nested, offset};

const ON_INNER: [f32; 2] = [10.0, 10.0];

#[test]
fn a_burst_keeps_the_container_it_started_on_past_its_end() {
    // Given: the inner list has 40 px to scroll
    let (doc, outer, inner) = nested();
    let mut wheel = WheelScroll::default();
    let start = Instant::now();

    // When: three notches' worth, each 40 ms after the last
    for step in 0..3_u32 {
        let now = start + Duration::from_millis(40) * step;
        wheel.scroll(&mut doc.borrow_mut(), ON_INNER, [0.0, 30.0], now);
    }

    // Then: the inner list stops at its end; the outer one never moves
    assert_eq!(offset(&doc, inner), 40.0);
    assert_eq!(offset(&doc, outer), 0.0);
}

#[test]
fn a_pause_longer_than_the_latch_finds_the_target_again() {
    // Given: the inner list scrolled to its end by one burst
    let (doc, outer, inner) = nested();
    let mut wheel = WheelScroll::default();
    let start = Instant::now();
    wheel.scroll(&mut doc.borrow_mut(), ON_INNER, [0.0, 50.0], start);

    // When: at the latch's end, then just past it
    wheel.scroll(&mut doc.borrow_mut(), ON_INNER, [0.0, 30.0], start + LATCH);
    let held = offset(&doc, outer);
    let later = start + LATCH + LATCH + Duration::from_millis(1);
    wheel.scroll(&mut doc.borrow_mut(), ON_INNER, [0.0, 30.0], later);

    // Then
    assert_eq!(offset(&doc, inner), 40.0);
    assert_eq!(held, 0.0);
    assert_eq!(offset(&doc, outer), 30.0);
}

#[test]
fn the_latch_is_what_chromium_measured() {
    assert_eq!(LATCH, Duration::from_millis(500));
}

#[test]
fn each_event_in_a_burst_restarts_the_latch() {
    // Given: the inner list at its end from the burst's first event
    let (doc, outer, inner) = nested();
    let mut wheel = WheelScroll::default();
    let start = Instant::now();
    wheel.scroll(&mut doc.borrow_mut(), ON_INNER, [0.0, 50.0], start);

    // When: two more, each 400 ms after the last: the third is 800 ms
    // after the first, but within the latch of the second
    for step in 1..3_u32 {
        let now = start + Duration::from_millis(400) * step;
        wheel.scroll(&mut doc.borrow_mut(), ON_INNER, [0.0, 30.0], now);
    }

    // Then
    assert_eq!(offset(&doc, inner), 40.0);
    assert_eq!(offset(&doc, outer), 0.0);
}
