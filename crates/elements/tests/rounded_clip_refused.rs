//! What a rounded clip refuses (flat-scene F3.2b, `clip_refusals.rs`):
//! a clip overlapping it without either holding the other, a scroller
//! holding it, an img or a gpu-surface under it, and a text field whose
//! text crosses its corner.

use crate::support;

use support::rounded_page::{CARD, at, card};
use support::{document, element, refusals};
use vapors_dom::Dom;
use vapors_elements::Document;
use vapors_scene::{RgbaImage, Size};

#[test]
fn a_clipping_box_across_a_rounded_corner_is_refused() {
    // Given: a clipping box over the card's top corners
    let css = ".inner { overflow: hidden; height: 20px; }";
    let (mut doc, _) = card(css, "inner");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert!(errors.iter().any(|e| e.contains("overlap")), "{errors:?}");
}

#[test]
fn a_scroll_container_holding_a_rounded_clip_is_refused() {
    // Given: a scroller larger than the card's padding box, around it
    let css = ".inner { overflow: auto; margin: -10px; height: 100px; }";
    let (mut doc, _) = card(css, "inner");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert!(errors.iter().any(|e| e.contains("scrolls")), "{errors:?}");
}

#[test]
fn an_img_under_a_rounded_clip_is_refused() {
    // Given
    let (mut doc, card) = card("img { display: block; }", "");
    let image = RgbaImage {
        size: Size {
            width: 4,
            height: 4,
        },
        rgba: [0, 0, 0, 255].repeat(16),
    };
    doc.register_image("a.png", image).expect("registers");
    let img = element(&mut doc, card, "img", "");
    doc.set_attribute(img, "src", "a.png");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert!(
        errors
            .iter()
            .any(|e| e.contains("is an img under a rounded clip")),
        "{errors:?}"
    );
}

#[test]
fn a_gpu_surface_under_a_rounded_clip_is_refused() {
    // Given
    let (mut doc, card) = card(
        "gpu-surface { display: block; width: 40px; height: 40px; }",
        "",
    );
    let surface = element(&mut doc, card, "gpu-surface", "");
    doc.set_attribute(surface, "name", "track");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert!(
        errors
            .iter()
            .any(|e| e.contains("is a gpu-surface under a rounded clip")),
        "{errors:?}"
    );
}

/// A card holding a text field styled `css`.
fn card_field(css: &str) -> Document {
    let mut doc = document();
    let body = doc.body();
    let card = element(&mut doc, body, "div", "card");
    let field = element(&mut doc, card, "input", "field");
    doc.set_attribute(field, "type", "text");
    doc.add_stylesheet(
        "page.css",
        &format!("{CARD} .field {{ display: block; {css} }}"),
    )
    .expect("parses");
    doc
}

#[test]
fn a_text_field_clear_of_a_rounded_clips_corners_is_not_refused() {
    // Given: a field that does not clip, 20px down, inset 10px
    let mut doc = card_field("overflow: visible; margin: 20px 10px; width: 60px;");

    // Then: the update is not refused
    doc.update(at(1.0)).expect("updates");
}

#[test]
fn a_text_field_whose_text_crosses_a_rounded_corner_is_refused() {
    // Given: a field that does not clip, its content box in the card's
    // top-left corner (one that clips is an overlap of clips, above)
    let mut doc = card_field("overflow: visible; border: 0; padding: 0; width: 60px;");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert!(
        errors.iter().any(|e| e.contains("its text crosses")),
        "{errors:?}"
    );
}
