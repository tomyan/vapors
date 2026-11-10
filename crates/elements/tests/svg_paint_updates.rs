//! An svg's icon after its attributes and colour change between two
//! updates (docs/icons.md, Paint): the second scene's item holds the new
//! icon and colour, not the first's.

mod support;

use support::svg_page::icon;
use support::viewport;
use vapors_dom::Dom;
use vapors_elements::Painted;
use vapors_scene::{Color, Item};
use vapors_text::{Icon, IconPath, IconStroke};

fn only_icon(painted: &Painted) -> (&Icon, Color) {
    let items = painted.scene().items();
    let found: Vec<_> = items
        .iter()
        .filter_map(|item| match *item {
            Item::Icon { icon, colour, .. } => Some((painted.icon(icon), colour)),
            _ => None,
        })
        .collect();
    assert_eq!(found.len(), 1, "{found:?}");
    found[0]
}

#[test]
fn changing_d_view_box_and_color_between_updates_changes_the_icon_and_colour() {
    // Given: an icon painted once
    let (mut doc, svg) = icon("color: rgb(1, 2, 3);", &[], &[]);
    let path = doc.children(svg)[0];
    doc.update(viewport(400.0, 300.0)).expect("updates");

    // When: all three change
    doc.set_attribute(path, "d", "M1 1L7 7");
    doc.set_attribute(svg, "viewBox", "0 0 8 8");
    doc.add_stylesheet("more.css", ".icon { color: rgb(9, 8, 7); }")
        .expect("parses");
    let painted = doc.update(viewport(400.0, 300.0)).expect("updates");

    // Then
    let (found, colour) = only_icon(painted);
    let stroke = IconStroke {
        path: IconPath::parse("M1 1L7 7").expect("parses"),
        width: 1.5,
    };
    let expected = Icon::new([0.0, 0.0, 8.0, 8.0], vec![stroke]).expect("a valid icon");
    assert_eq!(found, &expected);
    assert_eq!(colour, Color::from_rgba8(9, 8, 7, 255));
}
