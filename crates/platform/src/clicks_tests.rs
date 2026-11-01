//! Counting presses in a row: soon after and near the last, one more,
//! within this OS's fallback interval and distance.

use std::time::Duration;

use super::*;

/// The counts for presses at `(point, ms after the first)`.
fn counts(presses: &[([f32; 2], u64)]) -> Vec<u32> {
    let start = Instant::now();
    let mut clicks = Clicks::default();
    presses
        .iter()
        .map(|(at, ms)| {
            clicks
                .pressed(*at, start + Duration::from_millis(*ms))
                .get()
        })
        .collect()
}

#[test]
fn presses_soon_after_and_near_the_last_count_up() {
    // Given: three presses a fifth of a second apart, a pixel or two apart
    let presses = [([10.0, 10.0], 0), ([11.0, 9.0], 200), ([12.0, 10.0], 400)];

    // Then
    assert_eq!(counts(&presses), [1, 2, 3]);
}

#[test]
fn a_press_later_than_the_interval_starts_again() {
    // Given: a millisecond past the interval
    let late = u64::try_from(FALLBACK.0.as_millis()).expect("small") + 1;

    // Then
    assert_eq!(counts(&[([10.0, 10.0], 0), ([10.0, 10.0], late)]), [1, 1]);
}

#[test]
fn a_press_further_than_the_distance_starts_again() {
    // Given: each a pixel past the distance across, then down
    let past = 10.0 + FALLBACK.1 + 1.0;

    // Then
    assert_eq!(
        counts(&[([10.0, 10.0], 0), ([past, 10.0], 100), ([past, past], 200)]),
        [1, 1, 1]
    );
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
#[test]
fn linux_falls_back_to_gtks_defaults() {
    assert_eq!(FALLBACK, (Duration::from_millis(400), 5.0));
}
