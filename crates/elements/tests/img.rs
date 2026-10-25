//! `img` (F2c S10): an element showing an image registered with the
//! document by name, sized by its natural size and aspect ratio.

mod support;

use support::{document, element, refusals, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{BoxRect, Document};
use vapors_scene::{RgbaImage, Size};

/// A `width` x `height` image, every pixel `rgba`.
fn image(width: u32, height: u32, rgba: [u8; 4]) -> RgbaImage {
    RgbaImage {
        size: Size { width, height },
        rgba: rgba.repeat((width * height) as usize),
    }
}

/// An `img` with `src` (when given) in a body with `css`.
fn img(src: Option<&str>, css: &str) -> (Document, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let img = element(&mut doc, body, "img", "pic");
    if let Some(src) = src {
        doc.set_attribute(img, "src", src);
    }
    let css = format!("body {{ margin: 0; }} .pic {{ display: block; {css} }}");
    doc.add_stylesheet("app.css", &css).expect("parses");
    (doc, img)
}

fn border_box(doc: &mut Document, node: NodeId) -> BoxRect {
    doc.update(viewport(400.0, 300.0))
        .expect("updates")
        .border_box(node)
        .expect("a box")
}

fn size(rect: BoxRect) -> [f32; 2] {
    [rect.width, rect.height]
}

#[test]
fn an_image_takes_its_natural_size() {
    // Given
    let (mut doc, img) = img(Some("a.png"), "");
    doc.register_image("a.png", image(40, 20, [255, 0, 0, 255]));

    // When / Then
    assert_eq!(size(border_box(&mut doc, img)), [40.0, 20.0]);
}

#[test]
fn one_length_sets_the_other_by_the_aspect_ratio() {
    // Given
    let (mut wide, wide_img) = img(Some("a.png"), "width: 80px;");
    let (mut tall, tall_img) = img(Some("a.png"), "height: 50px;");
    for doc in [&mut wide, &mut tall] {
        doc.register_image("a.png", image(40, 20, [255, 0, 0, 255]));
    }

    // When / Then
    assert_eq!(size(border_box(&mut wide, wide_img)), [80.0, 40.0]);
    assert_eq!(size(border_box(&mut tall, tall_img)), [100.0, 50.0]);
}

#[test]
fn an_img_without_src_is_an_empty_box_unless_sized() {
    // Given
    let (mut bare, bare_img) = img(None, "");
    let (mut sized, sized_img) = img(None, "width: 30px; height: 10px;");

    // When / Then
    assert_eq!(size(border_box(&mut bare, bare_img)), [0.0, 0.0]);
    assert_eq!(size(border_box(&mut sized, sized_img)), [30.0, 10.0]);
}

#[test]
fn an_unregistered_src_is_refused_naming_it() {
    // Given
    let (mut doc, _img) = img(Some("missing.png"), "");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert!(
        errors
            .iter()
            .any(|e| e.contains("missing.png") && e.contains("register_image")),
        "{errors:?}"
    );
}

#[test]
fn registering_an_image_lays_its_imgs_out_again() {
    // Given: laid out with one image, nothing pending
    let (mut doc, img) = img(Some("a.png"), "");
    doc.register_image("a.png", image(40, 20, [255, 0, 0, 255]));
    border_box(&mut doc, img);
    doc.take_changed();

    // When: another under the same name
    doc.register_image("a.png", image(10, 30, [0, 0, 255, 255]));

    // Then
    assert!(doc.take_changed());
    assert_eq!(size(border_box(&mut doc, img)), [10.0, 30.0]);
}

#[test]
fn an_inline_img_is_refused_naming_the_fix() {
    // Given: an img left at its initial display, inline, in a paragraph
    let mut doc = document();
    let body = doc.body();
    let p = element(&mut doc, body, "p", "");
    let img = element(&mut doc, p, "img", "");
    doc.set_attribute(img, "src", "a.png");
    doc.register_image("a.png", image(4, 4, [0, 0, 0, 255]));

    // When
    let errors = refusals(&mut doc);

    // Then
    assert!(
        errors
            .iter()
            .any(|e| e.contains("<img") && e.contains("display: block")),
        "{errors:?}"
    );
}

#[test]
fn an_image_of_the_same_shape_but_another_size_lays_out_again() {
    // Given: laid out with a 40 x 20 image
    let (mut doc, img) = img(Some("a.png"), "");
    doc.register_image("a.png", image(40, 20, [255, 0, 0, 255]));
    border_box(&mut doc, img);

    // When: 80 x 40, the same aspect ratio
    doc.register_image("a.png", image(80, 40, [255, 0, 0, 255]));

    // Then
    assert_eq!(size(border_box(&mut doc, img)), [80.0, 40.0]);
}
