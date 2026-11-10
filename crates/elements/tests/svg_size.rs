//! An svg icon's size as Chromium 155 lays it out: its `width` and
//! `height` attributes are CSS `width` and `height` (presentational
//! hints), so they are definite, follow `box-sizing`, and a flex item
//! keeps them unless it grows, shrinks or has a basis. Each expected
//! size was measured in headless Chromium 155 with the same markup.

mod support;

use support::svg_page::{PATH, SVG, with_attributes};
use support::{document, element, viewport};
use vapors_dom::NodeId;
use vapors_elements::Document;

/// A body holding a `div` (class `c`, styled `container`) holding the
/// icon (styled `icon`), and after it a sibling of `sibling` px by 5
/// when given.
fn page(container: &str, icon: &str, sibling: Option<u32>) -> (Document, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let div = element(&mut doc, body, "div", "c");
    let svg = element(&mut doc, div, "svg", "icon");
    with_attributes(&mut doc, svg, SVG);
    let path = element(&mut doc, svg, "path", "");
    with_attributes(&mut doc, path, PATH);
    if sibling.is_some() {
        element(&mut doc, div, "div", "sibling");
    }
    let wide = sibling.unwrap_or(0);
    let css = format!(
        "body {{ margin: 0; }} .c {{ {container} }} .icon {{ display: block; {icon} }} \
         .sibling {{ width: {wide}px; height: 5px; }}"
    );
    doc.add_stylesheet("app.css", &css).expect("parses");
    (doc, svg)
}

fn size(container: &str, icon: &str, sibling: Option<u32>) -> [f32; 2] {
    let (mut doc, svg) = page(container, icon, sibling);
    let painted = doc.update(viewport(400.0, 300.0)).expect("updates");
    let rect = painted.border_box(svg).expect("a box");
    [rect.width, rect.height]
}

#[test]
fn its_attributes_are_its_css_size_wherever_it_is_laid_out() {
    for (container, icon, sibling, chromium) in [
        ("display: flex; height: 50px", "", None, [16.0, 16.0]),
        (
            "display: flex; height: 50px",
            "align-self: stretch",
            None,
            [16.0, 16.0],
        ),
        (
            "display: flex; flex-direction: column; width: 100px",
            "",
            None,
            [16.0, 16.0],
        ),
        ("display: block", "", None, [16.0, 16.0]),
        (
            "display: grid; width: 100px; height: 50px",
            "",
            None,
            [16.0, 16.0],
        ),
        (
            "display: grid; grid-template-columns: 4px; width: 100px",
            "",
            None,
            [16.0, 16.0],
        ),
    ] {
        // When / Then
        assert_eq!(
            size(container, icon, sibling),
            chromium,
            "{container} / {icon}"
        );
    }
}

#[test]
fn a_flex_item_grows_and_takes_a_basis_on_the_main_axis_only() {
    for (container, icon, sibling, chromium) in [
        (
            "display: flex; width: 400px",
            "flex-grow: 1",
            None,
            [400.0, 16.0],
        ),
        (
            "display: flex; width: 400px",
            "flex: 1 1 0",
            None,
            [400.0, 16.0],
        ),
        ("display: flex", "flex-basis: 24px", None, [24.0, 16.0]),
        (
            "display: flex; flex-direction: column; height: 50px",
            "flex-grow: 1",
            None,
            [16.0, 50.0],
        ),
        (
            "display: flex; flex-direction: column",
            "flex-basis: 24px",
            None,
            [16.0, 24.0],
        ),
    ] {
        // When / Then
        assert_eq!(
            size(container, icon, sibling),
            chromium,
            "{container} / {icon}"
        );
    }
}

/// Chromium gives an svg flex item no automatic minimum size: it
/// shrinks by its basis like any item, down to its padding and border.
#[test]
fn a_flex_item_shrinks_on_the_main_axis_with_no_minimum() {
    for (container, icon, sibling, chromium) in [
        ("display: flex; width: 10px", "", None, [10.0, 16.0]),
        (
            "display: flex; width: 10px",
            "",
            Some(50),
            [2.421_875, 16.0],
        ),
        ("display: flex; width: 10px", "", Some(500), [0.312_5, 16.0]),
        (
            "display: flex; width: 10px",
            "padding: 2px",
            None,
            [10.0, 20.0],
        ),
        (
            "display: flex; width: 10px",
            "box-sizing: border-box; padding: 2px",
            None,
            [10.0, 16.0],
        ),
        (
            "display: flex; flex-direction: column; height: 10px",
            "",
            None,
            [16.0, 10.0],
        ),
        (
            "display: flex; flex-direction: column; height: 10px",
            "",
            Some(50),
            [16.0, 7.625],
        ),
    ] {
        // When
        let size = size(container, icon, sibling);

        // Then: Chromium rounds to its layout unit, 1/64 px
        let near = (0..2).all(|axis| (size[axis] - chromium[axis]).abs() <= 1.0 / 64.0);
        assert!(
            near,
            "{container} / {icon}: {size:?}, Chromium {chromium:?}"
        );
    }
}

#[test]
fn padding_and_border_follow_box_sizing() {
    for (icon, chromium) in [
        ("padding: 2px; border: 1px solid", [22.0, 22.0]),
        ("box-sizing: border-box; padding: 2px", [16.0, 16.0]),
        (
            "box-sizing: border-box; padding: 2px; border: 1px solid",
            [16.0, 16.0],
        ),
    ] {
        // When / Then
        assert_eq!(size("display: flex", icon, None), chromium, "{icon}");
    }
}

#[test]
fn an_icon_beside_a_label_in_a_flex_button_keeps_its_size() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let button = element(&mut doc, body, "button", "");
    let svg = element(&mut doc, button, "svg", "icon");
    with_attributes(&mut doc, svg, SVG);
    let path = element(&mut doc, svg, "path", "");
    with_attributes(&mut doc, path, PATH);
    let span = element(&mut doc, button, "span", "");
    support::text(&mut doc, span, "Export");
    doc.add_stylesheet(
        "app.css",
        "button { display: flex; } .icon { display: block; }",
    )
    .expect("parses");

    // When
    let painted = doc.update(viewport(400.0, 300.0)).expect("updates");

    // Then
    let rect = painted.border_box(svg).expect("a box");
    assert_eq!([rect.width, rect.height], [16.0, 16.0]);
}
