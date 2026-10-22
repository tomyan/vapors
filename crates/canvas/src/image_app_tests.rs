//! When the app asks for a file, and what it does with one.

use super::*;
use vapors_scene::Size;

const BACKGROUND: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};

fn one_pixel_png() -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, 1, 1);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("header");
    writer.write_image_data(&[1, 2, 3, 4]).expect("data");
    drop(writer);
    bytes
}

fn picks(app: &ImageApp, event: &Event) -> bool {
    app.opens_picker(event)
}

#[test]
fn o_opens_the_picker_with_or_without_an_image() {
    // Given
    let empty = ImageApp::new(BACKGROUND);
    let image = RgbaImage {
        size: Size {
            width: 1,
            height: 1,
        },
        rgba: vec![0; 4],
    };
    let showing = ImageApp::showing("dot.png", image, BACKGROUND);

    // When / Then
    assert!(picks(&empty, &Event::Character("o".into())));
    assert!(picks(&showing, &Event::Character("o".into())));
    assert!(picks(&showing, &Event::OpenRequested));
    assert!(!picks(&showing, &Event::Character("p".into())));
}

#[test]
fn a_tap_or_click_opens_the_picker_only_while_empty() {
    // Given
    let empty = ImageApp::new(BACKGROUND);
    let showing = ImageApp::showing(
        "dot.png",
        RgbaImage {
            size: Size {
                width: 1,
                height: 1,
            },
            rgba: vec![0; 4],
        },
        BACKGROUND,
    );
    let tap = Event::Tap { x: 1.0, y: 1.0 };
    let click = Event::PrimaryButton { pressed: false };

    // When / Then
    assert!(picks(&empty, &tap));
    assert!(picks(&empty, &click));
    assert!(!picks(&showing, &tap));
    assert!(!picks(&showing, &click));
    assert!(!picks(&empty, &Event::PrimaryButton { pressed: true }));
}

#[test]
fn an_opened_png_is_shown_fitted() {
    // Given
    let mut app = ImageApp::new(BACKGROUND);
    app.view.event(&Event::Resized {
        width: 300,
        height: 200,
    });

    // When
    app.file_opened("dot.png", &one_pixel_png());

    // Then
    assert!(app.view().has_image());
    assert_eq!(app.view().transform().scale, 200.0);
}

#[test]
fn the_label_follows_the_file_name_and_the_zoom() {
    // Given
    let mut app = ImageApp::new(BACKGROUND);
    app.view.event(&Event::Resized {
        width: 300,
        height: 200,
    });
    assert_eq!(app.label(), "no file  100%");

    // When
    app.file_opened("dot.png", &one_pixel_png());
    app.view.event(&Event::Character("1".into()));

    // Then
    assert_eq!(app.label(), "dot.png  100%");
    app.view.event(&Event::Character("+".into()));
    assert_eq!(app.label(), "dot.png  125%");
}

#[test]
fn a_file_that_is_not_a_png_leaves_the_view_empty() {
    // Given
    let mut app = ImageApp::new(BACKGROUND);

    // When
    app.file_opened("notes.txt", b"hello");

    // Then
    assert!(!app.view().has_image());
    assert_eq!(app.label(), "no file  100%");
}
