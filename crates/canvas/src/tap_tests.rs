//! The recogniser and the view together for taps: the photo's double
//! tap pairs taps, so four quick taps fit and then return.

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
