//! An inline `<svg>` icon (docs/icons.md): a replaced box of its
//! `width` and `height` attributes whose paths are not boxes, and in
//! the a11y tree as Chromium has it: an `aria-hidden` icon is left out,
//! so a button holding it keeps its own name; a `role="img"` icon is an
//! image named by its `aria-label`.

mod support;

use support::{document, element, text, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{AxNode, BoxRect, Document};

/// Adds touche's 16 x 16 export icon under `parent`, decorative or
/// named `label`; returns the svg and its path.
fn icon(doc: &mut Document, parent: NodeId, label: Option<&str>) -> (NodeId, NodeId) {
    let svg = element(doc, parent, "svg", "icon");
    for (name, value) in [("viewBox", "0 0 16 16"), ("width", "16"), ("height", "16")] {
        doc.set_attribute(svg, name, value);
    }
    match label {
        Some(label) => {
            doc.set_attribute(svg, "role", "img");
            doc.set_attribute(svg, "aria-label", label);
        }
        None => doc.set_attribute(svg, "aria-hidden", "true"),
    }
    text(doc, svg, "\n  ");
    let path = element(doc, svg, "path", "");
    for (name, value) in [
        ("d", "M8 10V2M5 5l3-3 3 3M3 9v5h10V9"),
        ("fill", "none"),
        ("stroke", "currentColor"),
        ("stroke-width", "1.5"),
    ] {
        doc.set_attribute(path, name, value);
    }
    (svg, path)
}

fn styled(doc: &mut Document, css: &str) {
    let css = format!("body {{ margin: 0; display: flex; }} .icon {{ display: block; }} {css}");
    doc.add_stylesheet("app.css", &css).expect("parses");
}

fn border_box(doc: &mut Document, node: NodeId) -> Option<BoxRect> {
    doc.update(viewport(400.0, 300.0))
        .expect("updates")
        .border_box(node)
}

#[test]
fn an_svg_takes_the_size_of_its_width_and_height_attributes() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let (svg, _) = icon(&mut doc, body, None);
    doc.set_attribute(svg, "viewBox", "0 0 16 8");
    doc.set_attribute(svg, "height", "12");
    doc.set_attribute(svg, "width", "24");
    styled(&mut doc, "");

    // When
    let rect = border_box(&mut doc, svg).expect("a box");

    // Then
    assert_eq!([rect.width, rect.height], [24.0, 12.0]);
}

#[test]
fn padding_and_border_are_around_its_natural_size() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let (svg, _) = icon(&mut doc, body, None);
    styled(&mut doc, ".icon { padding: 2px; border: 1px solid; }");

    // When
    let rect = border_box(&mut doc, svg).expect("a box");

    // Then
    assert_eq!([rect.width, rect.height], [22.0, 22.0]);
}

#[test]
fn its_paths_are_not_boxes_even_styled_as_blocks() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let (svg, path) = icon(&mut doc, body, None);
    styled(&mut doc, "path { display: block; height: 30px; }");

    // When
    let painted = doc.update(viewport(400.0, 300.0)).expect("updates");

    // Then
    assert_eq!(painted.border_box(path), None);
    let rect = painted.border_box(svg).expect("a box");
    assert_eq!([rect.width, rect.height], [16.0, 16.0]);
}

/// A button holding an icon and, when given, a label and its text.
fn button(label: Option<&str>, words: Option<&str>, icon_label: Option<&str>) -> Vec<AxNode> {
    let mut doc = document();
    let body = doc.body();
    let button = element(&mut doc, body, "button", "");
    if let Some(label) = label {
        doc.set_attribute(button, "aria-label", label);
    }
    icon(&mut doc, button, icon_label);
    if let Some(words) = words {
        let span = element(&mut doc, button, "span", "");
        text(&mut doc, span, words);
    }
    styled(&mut doc, "button { display: flex; }");
    doc.update(viewport(400.0, 300.0)).expect("updates");
    doc.ax_projection(&["button", "img"]).expect("projects")
}

fn node(role: &str, name: &str) -> AxNode {
    AxNode {
        role: role.to_owned(),
        name: name.to_owned(),
        value: None,
        properties: Vec::new(),
    }
}

#[test]
fn a_hidden_icon_is_left_out_and_the_button_keeps_its_label() {
    // When
    let tree = button(Some("Undo"), None, None);

    // Then
    assert_eq!(tree, [node("button", "Undo")]);
}

#[test]
fn a_hidden_icon_adds_nothing_to_a_name_from_content() {
    // When
    let tree = button(None, Some("Export"), None);

    // Then
    assert_eq!(tree, [node("button", "Export")]);
}

#[test]
fn a_named_icon_is_an_image_that_names_its_button() {
    // When
    let tree = button(None, None, Some("Export"));

    // Then
    assert_eq!(tree, [node("button", "Export"), node("img", "Export")]);
}
