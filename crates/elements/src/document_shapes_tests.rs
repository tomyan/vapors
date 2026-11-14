//! A document's shapes counted per update: an acquire made between two
//! updates is not the second update's tessellation.

use vapors_shapes::{FillRule, IconPath, Paint};

use crate::{Document, Viewport};

#[test]
fn an_acquire_between_updates_is_not_the_next_updates_tessellation() {
    // Given: a document updated once
    let mut doc = Document::new(vapors_text::FontSystem::bundle_only());
    let viewport = Viewport {
        width: 100.0,
        height: 100.0,
        scale: 1.0,
    };
    doc.update(viewport).expect("updates");

    // When: a path tessellated outside an update, then an update
    let path = IconPath::parse("M0 0L4 0L0 4z").expect("parses");
    let fill = Paint::Fill(FillRule::NonZero);
    doc.shapes
        .borrow_mut()
        .cache
        .acquire("test", &path, fill, 1.0)
        .expect("acquired");
    doc.update(viewport).expect("updates");

    // Then
    assert_eq!(doc.update_counts().shapes_tessellated, 0);
}

#[test]
fn an_svg_whose_repaint_paints_no_shape_releases_the_ones_it_had() {
    // Given: an svg whose one path is filled, so on the shapes route,
    // its fill live in the cache
    use vapors_dom::Dom;
    let mut doc = Document::new(vapors_text::FontSystem::bundle_only());
    let viewport = Viewport {
        width: 100.0,
        height: 100.0,
        scale: 1.0,
    };
    let svg = doc.create_element("svg");
    let size = [
        ("viewBox", "0 0 16 16"),
        ("width", "16"),
        ("height", "16"),
        ("aria-hidden", "true"),
        ("class", "icon"),
    ];
    for (name, value) in size {
        doc.set_attribute(svg, name, value);
    }
    let path = doc.create_element("path");
    for (name, value) in [("d", "M2 2h4v4z"), ("fill", "red")] {
        doc.set_attribute(path, name, value);
    }
    doc.append(svg, path);
    let body = doc.body();
    doc.append(body, svg);
    doc.add_stylesheet(
        "app.css",
        ".icon { display: block } .bare path { fill: none }",
    )
    .expect("parses");
    doc.update(viewport).expect("updates");
    assert_eq!(doc.shapes.borrow().cache.counts().0, 1);

    // When: a class leaves the path no fill and no stroke, which paints
    // nothing (the update refuses it)
    doc.set_attribute(svg, "class", "icon bare");
    let refused = doc.update(viewport);

    // Then: its shapes are released all the same: the svg's uses are
    // replaced with none (`use_for(svg, &[])`)
    assert!(refused.is_err());
    assert_eq!(doc.shapes.borrow().cache.counts().0, 0);
}
