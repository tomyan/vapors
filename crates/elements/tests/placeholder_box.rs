//! Where a placeholder is painted against its field's box
//! (`docs/placeholder.md`): not while hidden, cut to the content box
//! when the field's overflow is visible, centred when its line is
//! taller than the box.

use crate::support;

use support::{document, element, viewport};
use vapors_dom::Dom;
use vapors_scene::{Item, Rect};

/// The placeholder "Search" painted by a 100 px wide input at the top
/// left, styled by `css`: its origin, clip and line height, one per run.
fn placeholder(css: &str) -> Vec<([f32; 2], Option<Rect>, f32)> {
    let mut doc = document();
    let body = doc.body();
    let input = element(&mut doc, body, "input", "");
    doc.set_attribute(input, "placeholder", "Search");
    let sheet = format!(
        "body {{ margin: 0; }} input {{ display: block; box-sizing: border-box; width: 100px; margin: 0; border: none; font-size: 14px; {css} }}"
    );
    doc.add_stylesheet("a.css", &sheet).expect("parses");
    let painted = doc.update(viewport(200.0, 100.0)).expect("updates");
    let scene = painted.scene().to_scene();
    let mut found = Vec::new();
    for (item, clip) in scene.items().iter().zip(scene.clips()) {
        if let Item::Text { text, origin, .. } = item {
            let layout = painted.text(*text);
            if layout.text() == "Search" {
                found.push((*origin, clip.map(|clip| clip.rect()), layout.height()));
            }
        }
    }
    found
}

#[test]
fn a_hidden_field_paints_no_placeholder() {
    // Given
    let css = "height: 24px; padding: 2px;";

    // When
    let shown = placeholder(css).len();
    let hidden = placeholder(&format!("{css} visibility: hidden;")).len();

    // Then: the same field paints one when it is not hidden
    assert_eq!((shown, hidden), (1, 0));
}

#[test]
fn a_field_whose_overflow_is_visible_still_cuts_its_placeholder_to_the_content_box() {
    // Given, when: content box x 4 to 96, y 2 to 22
    let found = placeholder("height: 24px; padding: 2px 4px; overflow: visible;");

    // Then
    let [(_, clip, _)] = found[..] else {
        panic!("one placeholder, got {found:?}");
    };
    assert_eq!(clip, Some(Rect::new(4.0, 2.0, 96.0, 22.0)));
}

#[test]
fn a_line_taller_than_the_content_box_is_centred_on_it() {
    // Given, when: a 10 px content box at y 0, a 14 px font's taller line
    let found = placeholder("height: 10px; padding: 0;");

    // Then: the line starts above the box by half the overflow
    let [(origin, _, line)] = found[..] else {
        panic!("one placeholder, got {found:?}");
    };
    assert!(line > 10.0, "a line taller than the box: {line}");
    assert_eq!(origin[1], (10.0 - line) / 2.0);
}
