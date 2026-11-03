//! Keeping the view across a resize (tomyan/vapors#124): with it on, a
//! resize keeps the person's zoom and position, as when a phone's
//! keyboard shortens the window; with it off, the default, a resize
//! fits the image again.

use super::tests::view;
use super::*;
use vapors_platform::Event;

fn zoomed_in() -> ImageView {
    let mut view = view();
    view.zoom([200.0, 200.0], 2.0);
    view
}

#[test]
fn a_resize_fits_the_image_again_by_default() {
    // Given
    let mut view = zoomed_in();

    // When: the window shortens
    view.event(&Event::Resized {
        width: 400,
        height: 200,
    });

    // Then: fitted to 400 by 200
    assert_eq!(view.transform().scale, 2.0);
    assert_eq!(view.transform().translate, [0.0, 0.0]);
}

#[test]
fn with_the_view_kept_a_resize_keeps_the_zoom_and_position() {
    // Given
    let mut view = zoomed_in();
    let before = view.transform();
    view.keep_view_on_resize(true);

    // When
    view.event(&Event::Resized {
        width: 400,
        height: 200,
    });

    // Then
    assert_eq!(view.transform(), before);
}

#[test]
fn turned_off_again_the_next_resize_fits() {
    // Given
    let mut view = zoomed_in();
    view.keep_view_on_resize(true);
    view.keep_view_on_resize(false);

    // When
    view.event(&Event::Resized {
        width: 400,
        height: 200,
    });

    // Then
    assert_eq!(view.transform().scale, 2.0);
}

#[test]
fn the_first_resize_fits_even_with_the_view_kept() {
    // Given: an image and no viewport yet
    let image = RgbaImage {
        size: Size {
            width: 200,
            height: 100,
        },
        rgba: vec![0; 200 * 100 * 4],
    };
    let mut view = ImageView::new(image, Color::from_rgba8(40, 40, 40, 255));
    view.keep_view_on_resize(true);

    // When
    view.event(&Event::Resized {
        width: 400,
        height: 400,
    });

    // Then: there was no view to keep
    assert_eq!(view.transform().scale, 2.0);
}
