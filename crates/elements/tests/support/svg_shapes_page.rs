//! The basic-shape tests' page: a body holding svgs, each holding
//! shape elements (`path`, `rect`, `circle`) with the attributes given,
//! and what an update paints of them.

#![allow(dead_code)] // each test file uses a subset

use vapors_dom::NodeId;
use vapors_elements::Document;
use vapors_scene::{Color, Item, ShapeKind};

use super::svg_page::{SVG, with_attributes};
use super::{document, element, viewport};

/// An element's attributes, name and value.
pub type Attributes<'a> = &'a [(&'a str, &'a str)];
/// A shape element: its tag and attributes.
pub type Shape<'a> = (&'a str, Attributes<'a>);

pub const RECT: &[(&str, &str)] = &[
    ("x", "2"),
    ("y", "3"),
    ("width", "12"),
    ("height", "8"),
    ("rx", "3"),
    ("ry", "1.5"),
    ("fill", "red"),
];
pub const CIRCLE: &[(&str, &str)] = &[("cx", "8"), ("cy", "8"), ("r", "5"), ("fill", "blue")];

/// A body holding one svg per entry of `svgs`, each holding a shape
/// element per `(tag, attributes)`; the svgs.
pub fn page(svgs: &[&[Shape]]) -> (Document, Vec<NodeId>) {
    let mut doc = document();
    let body = doc.body();
    let nodes = (svgs.iter())
        .map(|shapes| {
            let svg = element(&mut doc, body, "svg", "icon");
            with_attributes(&mut doc, svg, SVG);
            for (tag, attributes) in *shapes {
                let shape = element(&mut doc, svg, tag, "");
                with_attributes(&mut doc, shape, attributes);
            }
            svg
        })
        .collect();
    doc.add_stylesheet("app.css", ".icon { display: block; }")
        .expect("parses");
    (doc, nodes)
}

/// Each shape item's kind and colour, in paint order.
pub fn drawn(doc: &mut Document) -> Vec<(ShapeKind, Color)> {
    let painted = doc.update(viewport(400.0, 300.0)).expect("updates");
    (painted.scene().to_scene().items().iter())
        .filter_map(|item| match item {
            Item::Shape(shape) => Some((shape.kind(), shape.colour)),
            _ => None,
        })
        .collect()
}

pub fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgba8(r, g, b, 255)
}
