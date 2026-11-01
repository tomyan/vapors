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
                .pressed(Some(*at), start + Duration::from_millis(*ms))
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
    let late = u64::try_from(fallback().0.as_millis()).expect("small") + 1;

    // Then
    assert_eq!(counts(&[([10.0, 10.0], 0), ([10.0, 10.0], late)]), [1, 1]);
}

#[test]
fn a_press_further_than_the_distance_starts_again() {
    // Given: each a pixel past the distance across, then down
    let past = 10.0 + fallback().1 + 1.0;

    // Then
    assert_eq!(
        counts(&[([10.0, 10.0], 0), ([past, 10.0], 100), ([past, past], 200)]),
        [1, 1, 1]
    );
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
#[test]
fn linux_falls_back_to_gtks_defaults() {
    assert_eq!(fallback(), (Duration::from_millis(400), 5.0));
}

#[test]
fn a_browsers_platform_picks_its_fallback() {
    for (platform, fallback) in [
        ("Windows", WINDOWS),
        ("macOS", MACOS),
        (
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0) AppleWebKit/605.1.15",
            MACOS,
        ),
        (
            "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X)",
            MACOS,
        ),
        ("Linux", GTK),
        ("Android", ANDROID),
        (
            "Mozilla/5.0 (Linux; Android 16; Pixel 9 Pro) AppleWebKit/537.36",
            ANDROID,
        ),
        ("Chrome OS", GTK),
        ("", GTK),
    ] {
        assert_eq!(by_platform(platform), fallback, "{platform:?}");
    }
}
