//! What `img` refuses (F2c S10): images that cannot be drawn, at
//! registration; and, on any img, src or not, what vapors does not lay
//! out or draw yet.

mod support;

use support::{document, element, refusals};
use vapors_dom::Dom;
use vapors_elements::Document;
use vapors_scene::{RgbaImage, Size};

fn image(width: u32, height: u32) -> RgbaImage {
    RgbaImage {
        size: Size { width, height },
        rgba: [0, 0, 0, 255].repeat((width * height) as usize),
    }
}

#[test]
fn an_image_that_cannot_be_drawn_is_refused_at_registration_naming_it() {
    let mut doc = document();
    let short = RgbaImage {
        rgba: vec![0; 12],
        ..image(2, 2)
    };
    for (name, bad, says) in [
        ("empty.png", image(0, 10), "zero"),
        ("flat.png", image(10, 0), "zero"),
        ("huge.png", image(8193, 1), "8192"),
        ("short.png", short, "16 bytes"),
    ] {
        // When
        let error = doc.register_image(name, bad).expect_err("refused");

        // Then
        assert!(
            error.contains(name) && error.contains(says),
            "{name}: {error}"
        );
    }
}

/// An img with no src in a body with `css`, and the `attributes` given.
fn bare_img(css: &str, attributes: &[(&str, &str)]) -> Document {
    let mut doc = document();
    let body = doc.body();
    let p = element(&mut doc, body, "div", "holder");
    let img = element(&mut doc, p, "img", "pic");
    for (name, value) in attributes {
        doc.set_attribute(img, name, value);
    }
    doc.add_stylesheet("app.css", &format!(".pic {{ {css} }}"))
        .expect("parses");
    doc
}

#[test]
fn every_img_is_checked_with_or_without_src() {
    for (css, attributes, says) in [
        ("width: 10px; height: 10px;", &[][..], "inline"),
        (
            "display: block; border-radius: 3px;",
            &[][..],
            "border-radius",
        ),
        ("display: block; max-width: 50%;", &[][..], "max-width"),
        ("display: block; padding: 5%;", &[][..], "padding"),
        ("display: block;", &[("width", "40")][..], "width"),
        ("display: block;", &[("height", "40")][..], "height"),
        ("display: block;", &[("srcset", "a.png 2x")][..], "srcset"),
    ] {
        // Given
        let mut doc = bare_img(css, attributes);

        // When
        let errors = refusals(&mut doc);

        // Then
        assert!(
            errors
                .iter()
                .any(|e| e.contains("<img") && e.contains(says)),
            "{css} {attributes:?}: {errors:?}"
        );
    }
}

#[test]
fn an_image_is_unregistered_only_when_no_img_names_it() {
    // Given: a.png shown by an img
    let mut doc = document();
    let body = doc.body();
    let img = element(&mut doc, body, "img", "");
    doc.set_attribute(img, "src", "a.png");
    doc.register_image("a.png", image(4, 4)).expect("registers");

    // When / Then: refused while the img names it, naming the img
    let error = doc.unregister_image("a.png").expect_err("refused");
    assert!(error.contains("a.png") && error.contains("<img"), "{error}");

    // When / Then: once the img is gone, it goes
    doc.remove(img);
    assert_eq!(doc.unregister_image("a.png"), Ok(()));
    let again = doc.unregister_image("a.png").expect_err("refused");
    assert!(again.contains("no image"), "{again}");
}
