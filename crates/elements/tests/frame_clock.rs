//! The frame clock: `frame_time` and `frame_delta` are one pair of
//! app-wide signals, advanced once a frame, wanted while read.

use std::time::Duration;

use vapors_elements::{advance_frame_clock, frame_clock_wanted, frame_delta, frame_time};
use vapors_signals::{Owner, ReadSignal, Signal};

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

#[test]
fn no_call_means_no_clock_wanted_and_a_frame_changes_nothing() {
    advance_frame_clock(ms(5));
    assert!(!frame_clock_wanted());
}

#[test]
fn two_calls_return_one_signal() {
    let owner = Owner::new();
    let (a, b) = owner.with(|| (frame_time(), frame_time()));
    owner.dispose();
    advance_frame_clock(ms(10));
    assert_eq!(a.get_untracked(), b.get_untracked());
    assert_eq!(frame_delta().get_untracked(), Duration::ZERO);
}

#[test]
fn time_is_since_the_first_frame_and_delta_since_the_previous() {
    let time = frame_time();
    let delta = frame_delta();
    advance_frame_clock(ms(100));
    assert_eq!(
        (time.get_untracked(), delta.get_untracked()),
        (ms(0), ms(0))
    );
    advance_frame_clock(ms(116));
    assert_eq!(
        (time.get_untracked(), delta.get_untracked()),
        (ms(16), ms(16))
    );
    advance_frame_clock(ms(150));
    assert_eq!(
        (time.get_untracked(), delta.get_untracked()),
        (ms(50), ms(34))
    );
}

#[test]
fn a_derived_value_follows_the_clock_and_wants_it_until_unmounted() {
    let owner = Owner::new();
    let doubled = owner.with(|| Signal::derived(|| frame_time().get() * 2));
    assert!(frame_clock_wanted(), "a live reader wants frames");
    advance_frame_clock(ms(1000));
    advance_frame_clock(ms(1010));
    assert_eq!(doubled.get_untracked(), ms(20));
    owner.dispose();
    assert!(!frame_clock_wanted(), "the last reader is gone");
    assert!(
        frame_time().try_get_untracked().is_ok(),
        "the clock outlives it"
    );
}

#[test]
fn a_reader_of_delta_alone_wants_the_clock() {
    let owner = Owner::new();
    let _ = owner.with(|| Signal::derived(|| frame_delta().get()));
    assert!(frame_clock_wanted());
    owner.dispose();
    assert!(!frame_clock_wanted());
}

#[test]
fn the_clock_is_read_only_to_a_script() {
    let (time, delta): (ReadSignal<Duration>, ReadSignal<Duration>) = (frame_time(), frame_delta());
    assert_eq!(time.get_untracked(), delta.get_untracked());
}
