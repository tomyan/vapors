//! The svg refusal tests' page: a body holding touche's export icon,
//! one `svg` (class `icon`, `display: block`) with one `path`, whose
//! attributes a test changes.

#![allow(dead_code)] // each test file uses a subset

use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;

use super::{document, element, refusals, viewport};

pub const SVG: &[(&str, &str)] = &[
    ("viewBox", "0 0 16 16"),
    ("width", "16"),
    ("height", "16"),
    ("aria-hidden", "true"),
];

pub const PATH: &[(&str, &str)] = &[
    ("d", "M2 8h11M9 4l4 4-4 4"),
    ("fill", "none"),
    ("stroke", "currentColor"),
    ("stroke-width", "1.5"),
];

/// `attributes` with `changes` applied: a value replaces, an empty one
/// removes.
fn changed<'a>(
    attributes: &[(&'a str, &'a str)],
    changes: &[(&'a str, &'a str)],
) -> Vec<(&'a str, &'a str)> {
    let mut out: Vec<_> = attributes
        .iter()
        .filter(|(name, _)| !changes.iter().any(|(changed, _)| changed == name))
        .copied()
        .collect();
    out.extend(changes.iter().filter(|(_, value)| !value.is_empty()));
    out
}

pub fn with_attributes(doc: &mut Document, node: NodeId, attributes: &[(&str, &str)]) {
    for (name, value) in attributes {
        doc.set_attribute(node, name, value);
    }
}

/// A body holding one `svg` (class `icon`, styled `css`) with one
/// `path`, each with the attribute changes given.
pub fn icon(css: &str, svg: &[(&str, &str)], path: &[(&str, &str)]) -> (Document, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let node = element(&mut doc, body, "svg", "icon");
    with_attributes(&mut doc, node, &changed(SVG, svg));
    let child = element(&mut doc, node, "path", "");
    with_attributes(&mut doc, child, &changed(PATH, path));
    doc.add_stylesheet("app.css", &format!(".icon {{ display: block; {css} }}"))
        .expect("parses");
    (doc, node)
}

/// Asserts the update is refused with one message holding every one of
/// `says`.
pub fn assert_refused(doc: &mut Document, says: &[&str]) {
    let errors = refusals(doc);
    assert!(
        errors
            .iter()
            .any(|error| says.iter().all(|s| error.contains(s))),
        "{says:?} in {errors:#?}"
    );
}

/// Asserts the update succeeds.
pub fn assert_accepted(doc: &mut Document) {
    let updated = doc.update(viewport(400.0, 300.0));
    assert!(updated.is_ok(), "{:?}", updated.err());
}
