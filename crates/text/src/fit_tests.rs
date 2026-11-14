//! The breaker and `fits` give one answer at the limit, and text
//! offered its own max-content width a rounding short stays on one line.

use super::*;
use crate::{Align, Family, FontSystem, Shaped, Style};
use parley::BaseDirection;

const TEXT: &str = "Clicked 17 times";

fn shaped(scale: f32) -> Shaped {
    let base = Style::new(Family::Geist, 15.0);
    let mut system = FontSystem::bundle_only();
    Shaped::new(&mut system, TEXT, &base, &[], scale, BaseDirection::Ltr)
}

/// Lines when `shaped` is broken at `width` device pixels at scale 1.
fn lines_at(shaped: &Shaped, width: f32) -> usize {
    shaped.lay_out(Some(width), Align::Start).lines().count()
}

/// The available width whose limit is exactly `content`.
fn available_with_limit(content: f32) -> f32 {
    let mut available = content - FIT_SLACK;
    while fit_limit(available) > content {
        available = available.next_down();
    }
    while fit_limit(available) < content {
        available = available.next_up();
    }
    available
}

#[test]
fn a_line_exactly_at_the_limit_fits_in_the_breaker_and_in_fits() {
    // Given
    let shaped = shaped(1.0);
    let content = shaped.lay_out(None, Align::Start).width();
    let available = available_with_limit(content);
    assert_eq!(fit_limit(available), content, "the limit is the content");

    // When
    let breaker = lines_at(&shaped, available) == 1;

    // Then
    assert_eq!((breaker, fits(content, available)), (true, true));
}

#[test]
fn a_line_one_ulp_past_the_limit_fits_in_neither() {
    // Given
    let shaped = shaped(1.0);
    let content = shaped.lay_out(None, Align::Start).width();
    let available = available_with_limit(content).next_down();
    assert_eq!(fit_limit(available).next_up(), content, "one ulp past");

    // When
    let breaker = lines_at(&shaped, available) == 1;

    // Then
    assert_eq!((breaker, fits(content, available)), (false, false));
}

#[test]
fn text_offered_its_max_content_width_a_rounding_short_stays_on_one_line() {
    for scale in [1.0, 2.0, 2.625, 2.75, 2.8125, 3.0] {
        // Given: the CSS width back from device pixels, one ulp short
        let shaped = shaped(scale);
        let max_content = shaped.lay_out(None, Align::Start).width() / scale;

        // When
        let laid_out = shaped.lay_out(Some(max_content.next_down()), Align::Start);

        // Then
        assert_eq!(laid_out.lines().count(), 1, "at {scale}");
    }
}

#[test]
fn a_number_that_is_not_finite_never_fits() {
    for (content, available) in [
        (10.0, f32::NAN),
        (f32::NAN, 100.0),
        (f32::NAN, f32::NAN),
        (10.0, f32::INFINITY),
        (f32::INFINITY, f32::INFINITY),
    ] {
        assert!(!fits(content, available), "{content} in {available}");
    }
    assert!(fits(10.0, 10.0));
}
