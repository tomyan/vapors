//! `img` in the scene (F2c S10): its image over its content box, after
//! its background and border, and nothing for an img without one.

mod support;

use support::{document, element, refusals, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;
use vapors_scene::{Item, Rect, RgbaImage, Size};

fn image(width: u32, height: u32) -> RgbaImage {
    RgbaImage {
        size: Size { width, height },
        rgba: [9, 8, 7, 255].repeat((width * height) as usize),
    }
}

fn shown(css: &str, src: Option<&str>) -> (Document, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let img = element(&mut doc, body, "img", "pic");
    if let Some(src) = src {
        doc.set_attribute(img, "src", src);
    }
    doc.register_image("a.png", image(40, 20));
    let css = format!("body {{ margin: 0; }} .pic {{ display: block; {css} }}");
    doc.add_stylesheet("app.css", &css).expect("parses");
    (doc, img)
}

/// The image items' rects at `scale`, and their images' sizes.
fn images(doc: &mut Document, scale: f32) -> Vec<(Rect, Size)> {
    let painted = doc
        .update(vapors_elements::Viewport {
            scale,
            ..viewport(200.0, 200.0)
        })
        .expect("updates");
    painted
        .scene()
        .items()
        .iter()
        .filter_map(|item| match item {
            Item::Image { image, rect } => Some((*rect, painted.image(*image).size)),
            _ => None,
        })
        .collect()
}

#[test]
fn an_img_draws_its_image_over_its_content_box_in_device_pixels() {
    // Given: 4 px padding and a 2 px border around a 40 x 20 image
    let (mut doc, _img) = shown("padding: 4px; border: 2px solid black;", Some("a.png"));

    // When
    let found = images(&mut doc, 2.0);

    // Then: the content box starts 6 px in, at 2x
    assert_eq!(
        found,
        [(
            Rect::new(12.0, 12.0, 92.0, 52.0),
            Size {
                width: 40,
                height: 20
            }
        )]
    );
}

#[test]
fn the_image_paints_after_the_imgs_background() {
    // Given
    let (mut doc, _img) = shown("background-color: red;", Some("a.png"));

    // When
    let painted = doc.update(viewport(200.0, 200.0)).expect("updates");
    let kinds: Vec<&str> = painted
        .scene()
        .items()
        .iter()
        .map(|item| match item {
            Item::Quad(_) => "quad",
            Item::Image { .. } => "image",
            _ => "other",
        })
        .collect();

    // Then
    assert_eq!(kinds, ["quad", "image"]);
}

#[test]
fn an_img_without_an_image_or_hidden_draws_none() {
    // Given
    let (mut bare, _) = shown("width: 30px; height: 10px;", None);
    let (mut hidden, _) = shown("visibility: hidden;", Some("a.png"));

    // When / Then
    assert_eq!(images(&mut bare, 1.0), []);
    assert_eq!(images(&mut hidden, 1.0), []);
}

#[test]
fn a_rounded_img_is_refused() {
    // Given
    let (mut doc, _img) = shown("border-radius: 4px;", Some("a.png"));

    // When
    let errors = refusals(&mut doc);

    // Then
    assert!(
        errors
            .iter()
            .any(|e| e.contains("<img") && e.contains("border-radius")),
        "{errors:?}"
    );
}
