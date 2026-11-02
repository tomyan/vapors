//! A pointer gesture that starts outside the area (a press and its drag,
//! a wheel, a touch and its pan and pinch, a double tap) moves nothing and
//! is answered `OutsideArea`; the same gesture inside the area steers.

use std::num::NonZeroU32;

use vapors_platform::{Event, TouchPhase};
use vapors_scene::{Color, Rect, Size};

use super::Answered;
use crate::{ImageView, Insets, RgbaImage};

/// A 200x100 image in a 400x400 frame with a 200 px panel on the right:
/// the area is x 0..200.
pub(super) fn panel_right() -> ImageView {
    let image = RgbaImage {
        size: Size {
            width: 200,
            height: 100,
        },
        rgba: vec![0; 200 * 100 * 4],
    };
    let mut view = ImageView::new(image, Color::from_rgba8(40, 40, 40, 255));
    view.event(&Event::Resized {
        width: 400,
        height: 400,
    });
    view.set_insets(Insets {
        right: 200,
        ..Insets::default()
    });
    view
}

pub(super) const IN_AREA: [f32; 2] = [100.0, 200.0];
pub(super) const IN_PANEL: [f32; 2] = [300.0, 200.0];

fn drag_from(view: &mut ImageView, from: [f32; 2]) -> Answered {
    view.event(&Event::PointerMoved {
        x: from[0],
        y: from[1],
    });
    let pressed = view.event(&Event::PrimaryButton {
        pressed: true,
        clicks: NonZeroU32::MIN,
    });
    view.event(&Event::PointerMoved {
        x: from[0] - 40.0,
        y: from[1],
    });
    view.event(&Event::PrimaryButton {
        pressed: false,
        clicks: NonZeroU32::MIN,
    });
    pressed
}

pub(super) fn touch(view: &mut ImageView, phase: TouchPhase, at: [f32; 2]) -> Answered {
    view.event(&Event::Touch {
        id: 1,
        phase,
        x: at[0],
        y: at[1],
    })
}

#[test]
fn the_area_is_the_frame_less_its_insets() {
    // Given
    let view = panel_right();

    // Then
    assert_eq!(view.area_rect(), Rect::new(0.0, 0.0, 200.0, 400.0));
    assert!(view.area_contains(IN_AREA));
    assert!(!view.area_contains(IN_PANEL));
    assert!(!view.area_contains([200.0, 10.0]), "the right edge is out");
}

#[test]
fn a_drag_that_starts_in_the_inset_moves_nothing_and_says_so() {
    // Given
    let (mut outside, mut inside) = (panel_right(), panel_right());
    let fitted = outside.transform();

    // When
    let from_panel = drag_from(&mut outside, IN_PANEL);
    let from_area = drag_from(&mut inside, IN_AREA);

    // Then
    assert_eq!(from_panel, Answered::OutsideArea);
    assert_eq!(outside.transform(), fitted);
    assert_eq!(from_area, Answered::Done);
    assert_eq!(inside.transform().translate[0], fitted.translate[0] - 40.0);
}

#[test]
fn a_wheel_over_the_inset_zooms_nothing_and_says_so() {
    // Given
    let mut view = panel_right();
    let fitted = view.transform();

    // When
    view.event(&Event::PointerMoved {
        x: IN_PANEL[0],
        y: IN_PANEL[1],
    });
    let over_panel = view.event(&Event::Scroll { dx: 0.0, dy: -1.0 });
    let after_panel = view.transform();
    view.event(&Event::PointerMoved {
        x: IN_AREA[0],
        y: IN_AREA[1],
    });
    let over_area = view.event(&Event::Scroll { dx: 0.0, dy: -1.0 });

    // Then
    assert_eq!((over_panel, after_panel), (Answered::OutsideArea, fitted));
    assert_eq!(over_area, Answered::Done);
    assert!(view.transform().scale > fitted.scale);
}

#[test]
fn a_touch_that_lands_in_the_inset_neither_pans_nor_pinches_until_it_lifts() {
    // Given
    let mut view = panel_right();
    let fitted = view.transform();

    // When: a finger lands on the panel, then pans and pinches
    let landed = touch(&mut view, TouchPhase::Started, IN_PANEL);
    let panned = view.event(&Event::Pan { dx: -30.0, dy: 0.0 });
    let pinched = view.event(&Event::Pinch {
        x: 150.0,
        y: 200.0,
        factor: 2.0,
    });
    touch(&mut view, TouchPhase::Ended, IN_PANEL);
    let held = view.transform();
    touch(&mut view, TouchPhase::Started, IN_AREA);
    let in_area = view.event(&Event::Pan { dx: -30.0, dy: 0.0 });

    // Then
    assert_eq!([landed, panned, pinched], [Answered::OutsideArea; 3]);
    assert_eq!(held, fitted);
    assert_eq!(in_area, Answered::Done);
    assert_eq!(view.transform().translate[0], fitted.translate[0] - 30.0);
}

#[test]
fn a_double_tap_in_the_inset_does_not_toggle_the_fit() {
    // Given
    let mut view = panel_right();
    let fitted = view.transform();

    // When
    let answered = view.event(&Event::DoubleTap {
        x: IN_PANEL[0],
        y: IN_PANEL[1],
    });

    // Then
    assert_eq!(
        (answered, view.transform()),
        (Answered::OutsideArea, fitted)
    );
}
