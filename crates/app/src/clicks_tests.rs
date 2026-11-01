//! Counting presses in a row: soon after and near the last, one more.

use std::time::Duration;

use vapors_platform::Instant;

use super::*;

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
    assert_eq!(
        counts(&[([10.0, 10.0], 0), ([11.0, 9.0], 200), ([12.0, 10.0], 400)]),
        [1, 2, 3]
    );
}

#[test]
fn a_press_later_than_the_double_click_time_starts_again() {
    assert_eq!(counts(&[([10.0, 10.0], 0), ([10.0, 10.0], 501)]), [1, 1]);
}

#[test]
fn a_press_further_than_the_slop_starts_again() {
    assert_eq!(
        counts(&[([10.0, 10.0], 0), ([15.0, 10.0], 100), ([15.0, 15.0], 200)]),
        [1, 1, 1]
    );
}
