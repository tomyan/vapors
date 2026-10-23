//! How the view answers input.

use super::*;

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

#[test]
fn resize_fits_the_image() {
    // Given / When
    let view = view();

    // Then
    assert_eq!(view.transform().scale, 2.0);
    assert_eq!(view.transform().translate, [0.0, 100.0]);
}

#[test]
fn scroll_zooms_about_the_pointer() {
    // Given
    let mut view = view();
    view.event(&Event::PointerMoved { x: 100.0, y: 200.0 });
    let under_pointer = view.transform().image_point([100.0, 200.0]);

    // When
    view.event(&Event::Scroll { steps: 1.0 });

    // Then
    assert!((view.transform().scale - 2.5).abs() < 1e-5);
    let after = view.transform().image_point([100.0, 200.0]);
    assert!((after[0] - under_pointer[0]).abs() < 1e-4);
    assert!((after[1] - under_pointer[1]).abs() < 1e-4);
}

#[test]
fn scroll_before_any_pointer_move_zooms_about_the_view_centre() {
    // Given: winit's wheel event carries no position, and no pointer has moved
    let mut view = view();
    let under_centre = view.transform().image_point([200.0, 200.0]);

    // When
    view.event(&Event::Scroll { steps: 1.0 });

    // Then
    let after = view.transform().image_point([200.0, 200.0]);
    assert!(
        (after[0] - under_centre[0]).abs() < 1e-4,
        "{after:?} vs {under_centre:?}"
    );
    assert!(
        (after[1] - under_centre[1]).abs() < 1e-4,
        "{after:?} vs {under_centre:?}"
    );
}

#[test]
fn a_drag_does_not_pan_on_the_first_pointer_position() {
    // Given: the button goes down before any pointer position is known
    let mut view = view();
    let before = view.transform().translate;
    view.event(&Event::PrimaryButton { pressed: true });

    // When
    view.event(&Event::PointerMoved { x: 300.0, y: 300.0 });

    // Then
    assert_eq!(view.transform().translate, before);
}

#[test]
fn keys_fit_actual_size_and_zoom() {
    // Given
    let mut view = view();

    // When / Then
    view.event(&Event::Character("1".into()));
    assert_eq!(view.transform().scale, 1.0);
    view.event(&Event::Character("+".into()));
    assert!((view.transform().scale - 1.25).abs() < 1e-5);
    view.event(&Event::Character("0".into()));
    assert_eq!(view.transform().scale, 2.0);
}

#[test]
fn pinch_zooms_about_the_anchor() {
    // Given
    let mut view = view();
    let under_anchor = view.transform().image_point([100.0, 200.0]);

    // When
    view.event(&Event::Pinch {
        x: 100.0,
        y: 200.0,
        factor: 2.0,
    });

    // Then
    assert_eq!(view.transform().scale, 4.0);
    assert_eq!(view.transform().image_point([100.0, 200.0]), under_anchor);
}

#[test]
fn pan_translates_the_image() {
    // Given
    let mut view = view();

    // When
    view.event(&Event::Pan { dx: 5.0, dy: -3.0 });

    // Then
    assert_eq!(view.transform().translate, [5.0, 97.0]);
}

#[test]
fn double_tap_fits_unless_fitted_then_goes_to_actual_size() {
    // Given
    let mut view = view();
    view.event(&Event::Pan { dx: 5.0, dy: 0.0 });

    // When / Then
    view.event(&Event::DoubleTap { x: 50.0, y: 50.0 });
    assert_eq!(
        view.transform(),
        Transform::fit(view.image_size().unwrap(), view.viewport)
    );
    view.event(&Event::DoubleTap { x: 50.0, y: 50.0 });
    assert_eq!(view.transform().scale, 1.0);
}

#[test]
fn primary_button_drag_pans_and_release_stops_it() {
    // Given
    let mut view = view();
    view.event(&Event::PointerMoved { x: 100.0, y: 100.0 });
    view.event(&Event::PrimaryButton { pressed: true });

    // When
    view.event(&Event::PointerMoved { x: 110.0, y: 105.0 });
    view.event(&Event::PrimaryButton { pressed: false });
    view.event(&Event::PointerMoved { x: 200.0, y: 200.0 });

    // Then
    assert_eq!(view.transform().translate, [10.0, 105.0]);
}

#[test]
fn an_empty_view_ignores_fit_and_double_tap() {
    // Given
    let mut view = ImageView::empty(Color::from_rgba8(0, 0, 0, 255));
    view.event(&Event::Resized {
        width: 400,
        height: 400,
    });

    // When
    view.event(&Event::DoubleTap { x: 1.0, y: 1.0 });
    view.event(&Event::Character("0".into()));

    // Then
    assert!(!view.has_image());
    assert_eq!(view.transform().scale, 1.0);
}

#[test]
fn setting_an_image_fits_it() {
    // Given
    let mut view = ImageView::empty(Color::from_rgba8(0, 0, 0, 255));
    view.event(&Event::Resized {
        width: 400,
        height: 400,
    });

    // When
    view.set_image(RgbaImage {
        size: Size {
            width: 200,
            height: 100,
        },
        rgba: vec![0; 200 * 100 * 4],
    });

    // Then
    assert!(view.has_image());
    assert_eq!(view.transform().scale, 2.0);
}
