//! What an update paints: backgrounds as quads snapped to device
//! pixels, text runs, and the canvas colour from the root or body.

mod support;

use support::{document, element, text};
use vapors_dom::Dom;
use vapors_elements::Viewport;
use vapors_scene::{Color, Item, Quad, Rect};

#[test]
fn a_background_is_a_quad_with_edges_snapped_to_device_pixels() {
    // Given: a 10.5 px box 3.25 px from the left, 2 px tall, at 1.5x
    let mut doc = document();
    let body = doc.body();
    element(&mut doc, body, "div", "box");
    doc.add_stylesheet("app.css", "body { margin: 0; } .box { margin-left: 3.25px; width: 10.5px; height: 2px; background-color: #ff0000; }")
        .expect("parses");

    // When
    let painted = doc
        .update(Viewport {
            width: 100.0,
            height: 50.0,
            scale: 1.5,
        })
        .expect("updates");

    // Then: 4.875 rounds up to 5, 20.625 up to 21, 3 stays
    let quad = Quad {
        rect: Rect::new(5.0, 0.0, 21.0, 3.0),
        background: Color::from_rgba8(255, 0, 0, 255),
    };
    assert_eq!(painted.scene().items(), [Item::Quad(quad)]);
}

#[test]
fn text_is_painted_after_its_elements_background() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let panel = element(&mut doc, body, "div", "panel");
    text(&mut doc, panel, "Hello");
    doc.add_stylesheet(
        "app.css",
        ".panel { background-color: #222; padding: 4px; }",
    )
    .expect("parses");

    // When
    let painted = doc
        .update(Viewport {
            width: 100.0,
            height: 50.0,
            scale: 1.0,
        })
        .expect("updates");

    // Then
    let items = painted.scene().items();
    assert!(
        matches!(
            items,
            [
                Item::Quad(_),
                Item::Text {
                    origin: [12.0, 12.0],
                    ..
                }
            ]
        ),
        "{items:?}"
    );
}

#[test]
fn the_bodys_background_becomes_the_canvas_when_the_roots_is_transparent() {
    // Given
    let mut doc = document();
    doc.add_stylesheet("app.css", "body { background-color: #e9ecef; }")
        .expect("parses");

    // When
    let painted = doc
        .update(Viewport {
            width: 100.0,
            height: 50.0,
            scale: 1.0,
        })
        .expect("updates");

    // Then: the canvas takes it, and body paints no quad of its own
    assert_eq!(painted.canvas(), Color::from_rgba8(0xe9, 0xec, 0xef, 255));
    assert_eq!(painted.scene().items(), []);
}

#[test]
fn the_canvas_is_white_when_neither_root_nor_body_has_a_background() {
    // Given
    let mut doc = document();

    // When
    let painted = doc
        .update(Viewport {
            width: 100.0,
            height: 50.0,
            scale: 1.0,
        })
        .expect("updates");

    // Then
    assert_eq!(painted.canvas(), Color::from_rgba8(255, 255, 255, 255));
}

#[test]
fn an_element_with_display_none_has_no_box_and_paints_nothing() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let hidden = element(&mut doc, body, "div", "hidden");
    doc.add_stylesheet(
        "app.css",
        ".hidden { display: none; background-color: red; height: 5px; }",
    )
    .expect("parses");

    // When
    let painted = doc
        .update(Viewport {
            width: 100.0,
            height: 50.0,
            scale: 1.0,
        })
        .expect("updates");

    // Then
    assert_eq!(painted.border_box(hidden), None);
    assert_eq!(painted.scene().items(), []);
}
