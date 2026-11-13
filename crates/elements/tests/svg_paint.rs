//! An `svg` in the scene (docs/icons.md, Paint): one `Item::Icon` over
//! its content box snapped to device pixels, in its used `color`, under
//! its ancestors' clip; nothing when it is not visible.

mod support;

use support::svg_page::with_attributes;
use support::{document, element, viewport};
use vapors_dom::Dom;
use vapors_elements::{Document, Painted, Viewport};
use vapors_scene::{Color, IconId, Item, Rect};
use vapors_shapes::IconPath;
use vapors_text::{Icon, IconStroke};

const UNDO_ARROW: &str = "M5 3L2 6l3 3";
const UNDO_TAIL: &str = "M2 6h7a4 4 0 0 1 0 8H6";

/// A body styled `css` holding a `div.frame` holding touche's undo icon
/// as two paths, an `svg.icon` of 16 x 16.
fn undo_page(css: &str) -> Document {
    let mut doc = document();
    let body = doc.body();
    let frame = element(&mut doc, body, "div", "frame");
    let svg = element(&mut doc, frame, "svg", "icon");
    let size = [("viewBox", "0 0 16 16"), ("width", "16"), ("height", "16")];
    with_attributes(&mut doc, svg, &size);
    doc.set_attribute(svg, "aria-hidden", "true");
    for d in [UNDO_ARROW, UNDO_TAIL] {
        let path = element(&mut doc, svg, "path", "");
        let stroke = [("fill", "none"), ("stroke", "currentColor")];
        with_attributes(&mut doc, path, &[("d", d), ("stroke-width", "1.5")]);
        with_attributes(&mut doc, path, &stroke);
    }
    let css = format!("body {{ margin: 0; }} .icon {{ display: block; }} {css}");
    doc.add_stylesheet("app.css", &css).expect("parses");
    doc
}

fn updated(doc: &mut Document, scale: f32) -> &Painted {
    let viewport = Viewport {
        scale,
        ..viewport(200.0, 200.0)
    };
    doc.update(viewport).expect("updates")
}

/// The scene's icon items: id, rect, colour and clip.
fn icons(painted: &Painted) -> Vec<(IconId, Rect, Color, Option<Rect>)> {
    let scene = painted.scene().to_scene();
    let pairs = scene.items().iter().zip(scene.clips());
    pairs
        .filter_map(|(item, clip)| match *item {
            Item::Icon { icon, rect, colour } => Some((icon, rect, colour, *clip)),
            _ => None,
        })
        .collect()
}

fn undo_icon() -> Icon {
    let stroke = |d| IconStroke {
        path: IconPath::parse(d).expect("parses"),
        width: 1.5,
    };
    let strokes = vec![stroke(UNDO_ARROW), stroke(UNDO_TAIL)];
    Icon::new([0.0, 0.0, 16.0, 16.0], strokes).expect("a valid icon")
}

#[test]
fn an_svg_paints_its_paths_as_one_icon_item() {
    // Given
    let mut doc = undo_page("");

    // When
    let painted = updated(&mut doc, 1.0);

    // Then: the item names the svg's view box and its two paths
    let found = icons(painted);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(**painted.icon(found[0].0), undo_icon());
}

#[test]
fn an_svg_paints_over_its_content_box_snapped_to_device_pixels() {
    for (scale, expected) in [
        // 3.25 px margin, 2 px border, 1 px padding: content at 6.25
        (1.0, Rect::new(6.0, 6.0, 22.0, 22.0)),
        // 1.5x: edges 9.375 and 33.375 snap to 9 and 33
        (1.5, Rect::new(9.0, 9.0, 33.0, 33.0)),
        (2.0, Rect::new(13.0, 13.0, 45.0, 45.0)),
    ] {
        // Given
        let css = ".icon { margin: 3.25px; border: 2px solid black; padding: 1px; }";
        let mut doc = undo_page(css);

        // When
        let found = icons(updated(&mut doc, scale));

        // Then
        assert_eq!(found[0].1, expected, "at {scale}x");
    }
}

#[test]
fn an_svg_paints_in_the_color_it_inherits() {
    // Given: currentColor, the colour set on the svg's parent
    let mut doc = undo_page(".frame { color: rgb(10, 20, 30); }");

    // When
    let found = icons(updated(&mut doc, 1.0));

    // Then
    assert_eq!(found[0].2, Color::from_rgba8(10, 20, 30, 255));
}

#[test]
fn a_translucent_color_reaches_the_item_unchanged() {
    // Given
    let mut doc = undo_page(".icon { color: rgba(0, 0, 255, 0.5); }");

    // When
    let found = icons(updated(&mut doc, 1.0));

    // Then: compounding per path is the painter's (one quad per path)
    let Color { r, g, b, a } = found[0].2;
    assert_eq!([r, g, b, a], [0.0, 0.0, 1.0, 0.5]);
}

#[test]
fn a_hidden_svg_paints_no_icon() {
    // Given
    let mut doc = undo_page(".icon { visibility: hidden; }");

    // When
    let found = icons(updated(&mut doc, 1.0));

    // Then
    assert_eq!(found, []);
}

#[test]
fn a_visible_svg_inside_a_hidden_ancestor_paints() {
    // Given: visibility inherits, and the svg sets it back to visible
    let mut doc = undo_page(".frame { visibility: hidden; } .icon { visibility: visible; }");

    // When
    let found = icons(updated(&mut doc, 1.0));

    // Then
    assert_eq!(found.len(), 1, "{found:?}");
}

#[test]
fn an_svg_inside_a_clipping_ancestor_paints_under_its_clip() {
    // Given: a 10 x 10 frame that clips its 16 x 16 icon
    let mut doc = undo_page(".frame { width: 10px; height: 10px; overflow: hidden; }");

    // When
    let found = icons(updated(&mut doc, 2.0));

    // Then
    assert_eq!(found[0].3, Some(Rect::new(0.0, 0.0, 20.0, 20.0)));
}

#[test]
fn a_path_without_a_stroke_width_strokes_one_unit_wide() {
    // Given
    let (mut doc, _svg) = support::svg_page::icon("", &[], &[("stroke-width", "")]);

    // When
    let painted = updated(&mut doc, 1.0);

    // Then
    let found = icons(painted);
    let stroke = IconStroke {
        path: IconPath::parse(support::svg_page::PATH[0].1).expect("parses"),
        width: 1.0,
    };
    let expected = Icon::new([0.0, 0.0, 16.0, 16.0], vec![stroke]).expect("a valid icon");
    assert_eq!(**painted.icon(found[0].0), expected);
}
