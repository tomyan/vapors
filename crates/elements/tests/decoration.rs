//! Borders, radii, shadows and the cursor: what the scene holds for an
//! element, and what is refused.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::{document, element, refusals, viewport};
use vapors_elements::{Cursor, Document, Viewport};
use vapors_scene::Item;

fn painted_items(sheet: &str, scale: f32) -> Vec<Item> {
    let mut doc = document();
    let body = doc.body();
    element(&mut doc, body, "div", "box");
    doc.add_stylesheet("app.css", sheet).expect("parses");
    let viewport = Viewport {
        width: 200.0,
        height: 100.0,
        scale,
    };
    doc.update(viewport)
        .expect("updates")
        .scene()
        .items()
        .to_vec()
}

#[test]
fn a_border_is_drawn_inside_the_box_with_widths_snapped_to_device_pixels() {
    // Given: 0.5px (under a device pixel at 1x: one pixel) and 2.7px (floored to 2)
    let sheet = "body { margin: 0; } .box { width: 10px; height: 10px; border-top: 0.5px solid red; border-left: 2.7px solid blue; }";

    // When
    let items = painted_items(sheet, 1.0);

    // Then: the border box is 12.0 wide, 11 tall
    let [Item::Quad(quad)] = items.as_slice() else {
        panic!("one quad: {items:?}")
    };
    assert_eq!(quad.border, [1.0, 0.0, 0.0, 2.0]);
    assert_eq!((quad.rect.width(), quad.rect.height()), (12.0, 11.0));
}

#[test]
fn at_2x_a_half_pixel_border_is_one_device_pixel() {
    // Given
    let sheet = "body { margin: 0; } .box { width: 10px; height: 10px; border: 0.5px solid red; }";

    // When
    let items = painted_items(sheet, 2.0);

    // Then
    let [Item::Quad(quad)] = items.as_slice() else {
        panic!("one quad: {items:?}")
    };
    assert_eq!(quad.border, [1.0; 4]);
}

#[test]
fn shadows_come_before_the_box_with_the_first_declared_drawn_last() {
    // Given
    let sheet = "body { margin: 0; } .box { width: 10px; height: 10px; background-color: white; border-radius: 4px; box-shadow: 0 2px 4px red, 1px 1px 0 2px blue; }";

    // When
    let items = painted_items(sheet, 1.0);

    // Then
    let [Item::Shadow(blue), Item::Shadow(red), Item::Quad(_)] = items.as_slice() else {
        panic!("{items:?}")
    };
    assert_eq!(red.blur, 4.0);
    assert_eq!((red.rect.top, red.radii[0]), (2.0, 4.0));
    assert_eq!(
        (blue.rect.left, blue.rect.right),
        (-1.0, 13.0),
        "offset 1, spread 2"
    );
    assert_eq!(blue.radii[0], 6.0, "radii grow with the spread");
}

#[test]
fn unsupported_borders_corners_and_shadows_are_refused() {
    for (sheet, expected) in [
        (
            ".box { border: 1px dashed red; }",
            "`border-top-style: dashed`",
        ),
        (".box { border-radius: 50%; }", "is a percentage"),
        (".box { box-shadow: inset 0 0 2px red; }", "inset shadow"),
    ] {
        // Given
        let mut doc = document();
        let body = doc.body();
        element(&mut doc, body, "div", "box");
        doc.add_stylesheet("app.css", sheet).expect("parses");

        // When
        let errors = refusals(&mut doc);

        // Then
        assert!(
            errors.iter().any(|e| e.contains(expected)),
            "{sheet}: {errors:?}"
        );
    }
}

#[test]
fn the_cursor_is_the_hovered_elements_and_the_arrow_elsewhere() {
    // Given
    let mut doc = document();
    let body = doc.body();
    element(&mut doc, body, "div", "box");
    doc.add_stylesheet(
        "app.css",
        "body { margin: 0; } .box { height: 10px; cursor: pointer; }",
    )
    .expect("parses");
    doc.update(viewport(100.0, 100.0)).expect("updates");
    let doc = Rc::new(RefCell::new(doc));

    // When
    let (over, outside) = (
        doc.borrow().cursor([5.0, 5.0]),
        doc.borrow().cursor([5.0, 50.0]),
    );

    // Then
    assert_eq!((over, outside), (Cursor::Pointer, Cursor::Default));
    let _ = Document::hit;
}

#[test]
fn a_rounded_corner_where_borders_of_two_widths_meet_is_refused_naming_it() {
    // Given: the top-left corner is rounded and its borders are 2 and 4 px
    let mut doc = document();
    let body = doc.body();
    element(&mut doc, body, "div", "box");
    doc.add_stylesheet("app.css", ".box { border-top: 2px solid red; border-left: 4px solid red; border-top-left-radius: 8px; }")
        .expect("parses");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert!(
        error.starts_with("app.css:1:63: <div class=\"box\">: `border-top-left-radius: 8px`"),
        "{error}"
    );
    assert!(error.contains("elliptical inner corner"), "{error}");
}

#[test]
fn equal_borders_at_a_rounded_corner_and_unequal_ones_at_a_square_corner_are_drawn() {
    // Given: rounded top-left with equal widths; square bottom-right with 1 and 5
    let sheet = "body { margin: 0; } .box { width: 10px; height: 10px; border-top: 2px solid red; border-left: 2px solid red; border-bottom: 1px solid red; border-right: 5px solid red; border-top-left-radius: 6px; }";

    // When
    let items = painted_items(sheet, 1.0);

    // Then
    assert_eq!(items.len(), 1, "{items:?}");
}

#[test]
fn a_negative_shadow_blur_is_refused() {
    // Given
    let mut doc = document();
    let body = doc.body();
    element(&mut doc, body, "div", "box");
    doc.add_stylesheet("app.css", ".box { box-shadow: 0 2px -4px red; }")
        .expect("parses");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert!(
        errors
            .iter()
            .any(|e| e.contains("negative blur radius") && e.starts_with("app.css:1:")),
        "{errors:?}"
    );
}
