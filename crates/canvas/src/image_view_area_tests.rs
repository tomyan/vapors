//! Insets: the image is fitted, shown actual size and zoomed by key in
//! the frame less its insets (a side panel over the frame's right part),
//! and the frame keeps its full size for drawing.

use vapors_platform::Event;
use vapors_scene::{Color, Size};

use super::*;
use crate::{ImageView, RgbaImage};

/// A 200x100 image in a 400x400 frame.
fn view() -> ImageView {
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
    view
}

/// The 200x100 image in a 400x400 frame with a 200 px panel on the right.
fn panel_right() -> ImageView {
    let mut view = view();
    view.set_insets(Insets {
        right: 200,
        ..Insets::default()
    });
    view
}

#[test]
fn the_image_is_fitted_in_the_frame_less_its_insets() {
    // When
    let view = panel_right();

    // Then: 200 wide in the 200 wide area, centred in its 400 height
    assert_eq!(view.transform().scale, 1.0);
    assert_eq!(view.transform().translate, [0.0, 150.0]);
}

#[test]
fn a_left_or_top_inset_places_the_image_after_it() {
    // Given
    let mut view = view();

    // When: a panel on the left, a bar along the top
    view.set_insets(Insets {
        left: 200,
        top: 100,
        ..Insets::default()
    });

    // Then: in the 200x300 area from (200, 100)
    assert_eq!(view.transform().scale, 1.0);
    assert_eq!(view.transform().translate, [200.0, 200.0]);
}

#[test]
fn actual_size_and_the_zoom_keys_centre_on_the_area() {
    // Given
    let mut view = panel_right();

    // When
    view.actual_size();
    let actual = view.transform();
    view.event(&Event::Character("+".into()));

    // Then: 1:1 centred in the area; the zoom keeps the area's centre still
    assert_eq!(actual.translate, [0.0, 150.0]);
    assert_eq!(
        view.transform().image_point([100.0, 200.0]),
        actual.image_point([100.0, 200.0])
    );
}

#[test]
fn a_resize_refits_in_the_frame_less_its_insets() {
    // Given
    let mut view = panel_right();

    // When
    view.event(&Event::Resized {
        width: 600,
        height: 400,
    });

    // Then: 400 wide area, 2x
    assert_eq!(view.transform().scale, 2.0);
    assert_eq!(view.transform().translate, [0.0, 100.0]);
}

#[test]
fn insets_wider_than_the_frame_leave_an_empty_area_and_nothing_is_fitted() {
    // Given
    let mut view = view();

    // When
    view.set_insets(Insets {
        right: 500,
        ..Insets::default()
    });

    // Then: an empty area, as a minimised window: scale 0, nothing drawn
    assert_eq!(
        view.area_size(),
        Size {
            width: 0,
            height: 400
        }
    );
    assert_eq!(view.transform().scale, 0.0);
}
