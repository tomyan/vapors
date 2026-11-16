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

#[test]
fn each_container_fades_on_its_own_and_the_earliest_start_is_waited_for() {
    // Given two containers scrolled 100 ms apart
    let (mut fades, at) = (super::ThumbFades::default(), Instant::now());
    let (a, b) = (vapors_dom::NodeId(1), vapors_dom::NodeId(2));
    fades.scrolled(a, at);
    fades.scrolled(b, at + 100 * MS);

    // Then: each at its own opacity; shown until a's fade starts, then
    // fading while either does
    assert!((fades.opacity(a, at + 450 * MS) - 0.5).abs() < 1e-3);
    assert!((fades.opacity(b, at + 450 * MS) - 5.0 / 6.0).abs() < 1e-3);
    assert_eq!(fades.phase(at + 10 * MS), Phase::Shown(at + FADE_DELAY));
    assert_eq!(fades.phase(at + 650 * MS), Phase::Fading);
    assert_eq!(fades.phase(at + 700 * MS), Phase::Hidden);
}

#[test]
fn a_container_faded_out_is_forgotten() {
    // Given a container scrolled, its fade over
    let (doc, list, _) = crate::scroll_fixture::nested();
    let (mut fades, at) = (super::ThumbFades::default(), Instant::now());
    fades.scrolled(list, at);
    fades.apply(&mut doc.borrow_mut(), at);
    assert_eq!(fades.len(), 1);

    // When drawn at the fade's end
    fades.apply(&mut doc.borrow_mut(), at + FADE_DELAY + FADE_DURATION);

    // Then it is no longer visited each frame
    assert_eq!(fades.len(), 0);
}

#[test]
fn hiding_every_fade_draws_each_hidden_and_forgets_them() {
    // Given a container shown by a scroll
    let (doc, list, _) = crate::scroll_fixture::nested();
    let (mut fades, at) = (super::ThumbFades::default(), Instant::now());
    fades.scrolled(list, at);
    fades.apply(&mut doc.borrow_mut(), at);
    doc.borrow_mut().take_changed();

    // When the mouse takes over
    fades.hide_all(&mut doc.borrow_mut());

    // Then its own alpha is dropped once, and no fade is left
    assert!(doc.borrow_mut().take_changed());
    assert_eq!(fades.len(), 0);
    assert_eq!(fades.phase(at), Phase::Hidden);
}

#[test]
fn a_touch_after_the_mouse_draws_the_shared_thumbs_fading_from_full() {
    // Given the mouse's thumbs, drawn fully
    let (doc, _, _) = crate::scroll_fixture::nested();
    let (mut fades, at) = (super::ThumbFades::default(), Instant::now());
    doc.borrow_mut().take_changed();

    // When a finger touches, and frames draw the thumbs
    fades.touch_after_mouse(at);
    let mut changed_at = |when| {
        fades.apply(&mut doc.borrow_mut(), when);
        doc.borrow_mut().take_changed()
    };

    // Then they stay full through the delay, fade, and end hidden
    assert!(!changed_at(at), "not hidden at the touch");
    assert!(!changed_at(at + 299 * MS));
    assert!(changed_at(at + 450 * MS));
    assert!(changed_at(at + FADE_DELAY + FADE_DURATION));
    assert_eq!(fades.phase(at + FADE_DELAY + FADE_DURATION), Phase::Hidden);
}
