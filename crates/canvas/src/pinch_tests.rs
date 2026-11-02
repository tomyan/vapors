//! The recogniser and the view together: a real pinch keeps the image
//! point under the fingers where it is.

use std::time::Duration;

use vapors_platform::{Event, Gestures, TouchPhase};
use vapors_scene::{Color, Size};

use crate::{ImageView, RgbaImage};

fn view() -> ImageView {
    let image = RgbaImage {
        size: Size {
            width: 1000,
            height: 1000,
        },
        rgba: Vec::new(),
    };
    let mut view = ImageView::new(image, Color::from_rgba8(0, 0, 0, 255));
    view.event(&Event::Resized {
        width: 500,
        height: 500,
    });
    view
}

#[test]
fn a_pinch_keeps_the_image_point_under_the_midpoint_still() {
    // Given: finger A still at (100,100), finger B at (200,100)
    let mut view = view();
    let mut gestures = Gestures::new();
    let feed = |view: &mut ImageView, gestures: &mut Gestures, id, phase, x, ms| {
        for event in gestures.touch(id, phase, x, 100.0, Duration::from_millis(ms)) {
            view.event(&event);
        }
    };
    feed(&mut view, &mut gestures, 0, TouchPhase::Started, 100.0, 0);
    feed(&mut view, &mut gestures, 1, TouchPhase::Started, 200.0, 10);
    let under_midpoint = view.transform().image_point([150.0, 100.0]);

    // When: B moves from 200 to 400 in 10 px steps
    for step in 1..=20 {
        #[allow(clippy::cast_precision_loss)] // 20 steps
        let x = 200.0 + 10.0 * step as f32;
        feed(&mut view, &mut gestures, 1, TouchPhase::Moved, x, 10 + step);
    }

    // Then: the image point under the current midpoint (250,100) is the same
    let after = view.transform().image_point([250.0, 100.0]);
    assert!(
        (after[0] - under_midpoint[0]).abs() < 0.01 && (after[1] - under_midpoint[1]).abs() < 0.01,
        "image point under the midpoint moved from {under_midpoint:?} to {after:?}"
    );
    assert!((view.transform().scale - 1.5).abs() < 1e-4);
}

#[test]
fn four_quick_taps_fit_then_return_as_two_pairs() {
    // Given: a 1000x1000 image fitted in 500x500, then shown actual size
    let mut view = view();
    let mut gestures = Gestures::new();
    view.actual_size();
    let actual = view.transform();
    let mut tap = |view: &mut ImageView, n: u64| {
        let at = Duration::from_millis(n * 200);
        for (phase, t) in [
            (TouchPhase::Started, at),
            (TouchPhase::Ended, at + Duration::from_millis(50)),
        ] {
            for event in gestures.touch(n, phase, 250.0, 250.0, t) {
                view.event(&event);
            }
        }
    };

    // When: tap-tap, then tap-tap again (the photo's pairing, the lead,
    // review-v100 R2; not the DOM's 1, 2, 3 count)
    tap(&mut view, 0);
    tap(&mut view, 1);
    let after_first_pair = view.transform();
    tap(&mut view, 2);
    tap(&mut view, 3);

    // Then: fitted, then back to actual size
    assert_eq!(after_first_pair.scale, 0.5);
    assert_eq!(view.transform(), actual);
}
