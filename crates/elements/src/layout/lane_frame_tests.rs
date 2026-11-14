//! When a lane's children can be frames of their own.

use taffy::{Layout, Rect, Size};

use super::{FrameFacts, whole};
use crate::lane_decline::LaneDecline;

fn laid(height: f32, margins: [f32; 2]) -> Layout {
    Layout {
        size: Size {
            width: 100.0,
            height,
        },
        margin: Rect {
            left: 0.0,
            right: 0.0,
            top: margins[0],
            bottom: margins[1],
        },
        ..Layout::new()
    }
}

#[test]
fn whole_heights_and_origin_frame_the_children() {
    // Given
    let mut facts = FrameFacts::default();
    facts.note(1, &laid(20.0, [2.0, 3.0]), 1.0, false);
    facts.note(2, &laid(20.5, [0.0, 0.0]), 2.0, false);
    facts.top = 4.0;

    // Then: 20.5 is whole at scale 2
    assert_eq!(facts.framable(0.0, 10.0, 2.0), Ok(()));
}

#[test]
fn a_fractional_child_gap_or_origin_does_not() {
    let mut facts = FrameFacts::default();
    facts.note(1, &laid(20.0, [0.0, 0.0]), 1.0, false);
    assert_eq!(
        facts.framable(0.5, 0.0, 1.0),
        Err(LaneDecline::LaneFramePixel)
    );
    assert_eq!(
        facts.framable(0.0, 0.25, 1.0),
        Err(LaneDecline::LaneFramePixel)
    );
    facts.note(2, &laid(10.21, [0.0, 0.0]), 1.0, false);
    assert_eq!(
        facts.framable(0.0, 0.0, 1.0),
        Err(LaneDecline::LaneFramePixel)
    );

    // When: the fractional child is taken out, or laid out again whole
    facts.forget(2);

    // Then
    assert_eq!(facts.framable(0.0, 0.0, 1.0), Ok(()));
}

#[test]
fn a_child_holding_a_scroller_does_not_until_it_is_gone() {
    let mut facts = FrameFacts::default();
    facts.note(1, &laid(20.0, [0.0, 0.0]), 1.0, true);
    assert_eq!(
        facts.framable(0.0, 0.0, 1.0),
        Err(LaneDecline::LaneFrameScroller)
    );
    facts.note(1, &laid(20.0, [0.0, 0.0]), 1.0, false);
    assert_eq!(facts.framable(0.0, 0.0, 1.0), Ok(()));
}

#[test]
fn whole_means_whole_device_pixels() {
    assert!(whole(3.0, 1.0));
    assert!(whole(1.5, 2.0));
    assert!(!whole(1.25, 2.0));
    assert!(!whole(f64::NAN, 1.0));
}
