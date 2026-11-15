use std::time::Duration;

use vapors_platform::Instant;

use super::{FADE_DELAY, FADE_DURATION, Phase, ThumbFade};

const MS: Duration = Duration::from_millis(1);

#[test]
fn thumbs_never_scrolled_are_hidden_and_wait_for_nothing() {
    let fade = ThumbFade::default();
    let now = Instant::now();
    assert_eq!(fade.opacity(now), 0.0);
    assert_eq!(fade.phase(now), Phase::Hidden);
}

#[test]
fn a_scroll_shows_the_thumbs_at_once_for_300_ms() {
    // Given
    let (mut fade, at) = (ThumbFade::default(), Instant::now());

    // When
    fade.scrolled(at);

    // Then: full opacity at once (no fade in), until the delay ends
    assert_eq!(fade.opacity(at), 1.0);
    assert_eq!(fade.opacity(at + Duration::from_millis(299)), 1.0);
    assert_eq!(fade.phase(at + MS), Phase::Shown(at + FADE_DELAY));
}

#[test]
fn after_the_delay_the_thumbs_fade_linearly_over_300_ms() {
    // Given
    let (mut fade, at) = (ThumbFade::default(), Instant::now());
    fade.scrolled(at);
    let start = at + FADE_DELAY;

    // Then
    assert!((fade.opacity(start + FADE_DURATION / 2) - 0.5).abs() < 1e-3);
    assert_eq!(fade.phase(start + MS), Phase::Fading);
    assert_eq!(fade.opacity(start + FADE_DURATION), 0.0);
    assert_eq!(fade.phase(start + FADE_DURATION), Phase::Hidden);
}

#[test]
fn a_scroll_during_the_fade_shows_the_thumbs_again() {
    // Given a fade half done
    let (mut fade, at) = (ThumbFade::default(), Instant::now());
    fade.scrolled(at);
    let later = at + FADE_DELAY + FADE_DURATION / 2;

    // When
    fade.scrolled(later);

    // Then
    assert_eq!(fade.opacity(later), 1.0);
    assert_eq!(fade.phase(later), Phase::Shown(later + FADE_DELAY));
}
