//! A dialog holding one control of each role the a11y tree names, as
//! the corpus's `roles.wisp` has them (R1): a group of a range, a
//! number and a text field; a region of two checkboxes (the second
//! checked and disabled), a toggle button, a button and a disabled
//! button; and a section with no name holding one more button.

use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;

use super::{document, element, text, viewport};

/// The page's style: rows of fixed-size controls, 24 px tall.
pub const STYLE: &str = "body { margin: 0; } .row { display: flex; height: 24px; } \
    input, button { box-sizing: border-box; height: 24px; margin: 0; padding: 0; } \
    #amount { width: 200px; } #radius { width: 80px; } #note { width: 120px; } \
    button { width: 80px; }";

/// The page, updated at 640x400.
#[allow(clippy::too_many_lines)] // the fixture page, one literal per control
pub fn page() -> Document {
    let mut doc = document();
    let body = doc.body();
    let dialog = labelled(
        &mut doc,
        body,
        "div",
        &[("role", "dialog"), ("aria-label", "Levels")],
    );
    let group = labelled(
        &mut doc,
        dialog,
        "div",
        &[("class", "row"), ("role", "group"), ("aria-label", "Tone")],
    );
    labelled(
        &mut doc,
        group,
        "input",
        &[
            ("id", "amount"),
            ("type", "range"),
            ("aria-label", "Amount"),
            ("value", "50"),
        ],
    );
    labelled(
        &mut doc,
        group,
        "input",
        &[
            ("id", "radius"),
            ("type", "number"),
            ("aria-label", "Radius"),
            ("value", "2"),
        ],
    );
    labelled(
        &mut doc,
        group,
        "input",
        &[
            ("id", "note"),
            ("type", "text"),
            ("aria-label", "Note"),
            ("value", "hi"),
        ],
    );
    let region = labelled(
        &mut doc,
        dialog,
        "section",
        &[("class", "row"), ("aria-label", "Options")],
    );
    labelled(
        &mut doc,
        region,
        "input",
        &[("type", "checkbox"), ("aria-label", "Mono")],
    );
    labelled(
        &mut doc,
        region,
        "input",
        &[
            ("type", "checkbox"),
            ("aria-label", "Lock"),
            ("checked", ""),
            ("disabled", ""),
        ],
    );
    button(&mut doc, region, "Pin", &[("aria-pressed", "false")]);
    button(&mut doc, region, "Apply", &[]);
    button(&mut doc, region, "Reset", &[("disabled", "")]);
    let unnamed = element(&mut doc, dialog, "section", "row");
    button(&mut doc, unnamed, "After", &[]);
    doc.add_stylesheet("roles.css", STYLE).expect("parses");
    doc.update(viewport(640.0, 400.0)).expect("updates");
    doc
}

/// An element with `tag` and `attributes`, appended to `parent`.
pub fn labelled(
    doc: &mut Document,
    parent: NodeId,
    tag: &str,
    attributes: &[(&str, &str)],
) -> NodeId {
    let node = element(doc, parent, tag, "");
    for (name, value) in attributes {
        doc.set_attribute(node, name, value);
    }
    node
}

/// A button with `label` as its content.
pub fn button(
    doc: &mut Document,
    parent: NodeId,
    label: &str,
    attributes: &[(&str, &str)],
) -> NodeId {
    let node = labelled(doc, parent, "button", attributes);
    text(doc, node, label);
    node
}
