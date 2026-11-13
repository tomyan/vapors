//! The scroller pages: scroll frames inside a scroll frame, by the
//! thousand. IU4's per-scroll-frame extents index searches every frame
//! on each sort (docs/plans/iu4.md, "PR 5 as built", M2, still open),
//! and these pages show what that costs.

use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;

use crate::off_path_page::{Shape, Unit, value_text};
use crate::rows::basics::{element, text};

/// A scroller of rows (the body), each row a horizontal scroller of
/// four 200 px cells: 800 px of cells in a 640 px row. Both axes are
/// `auto`: vapors refuses axes that differ (`overflow-x: auto;
/// overflow-y: hidden`), and the 20 px cells leave nothing to scroll
/// down.
pub const NESTED_SCROLLERS: Shape = Shape {
    css: ".scroller { display: flex; height: 24px; overflow: auto; } \
          .cell { flex-shrink: 0; width: 200px; height: 20px; }",
    container: body,
    unit: scroller_row,
};

/// A transcript (the body) of messages, each a line of prose and a
/// code block of four 900 px lines in a 48 px block: a small scroller
/// that moves both ways. Each line is its own block, wide enough that
/// its text does not wrap: vapors refuses `white-space: pre` and
/// `nowrap`.
pub const CODE_BLOCKS: Shape = Shape {
    css: ".message { padding: 8px; } \
          .code { height: 48px; overflow: auto; font-size: 13px; \
                  font-family: \"Geist Mono\"; background-color: #f4f4f4; } \
          .line { width: 900px; height: 18px; }",
    container: body,
    unit: message,
};

/// The body: the pages' units are its children.
fn body(doc: &mut Document) -> NodeId {
    doc.body()
}

/// An element with `class` made under `parent`, before `before` or last.
fn child_before(
    doc: &mut Document,
    parent: NodeId,
    tag: &str,
    class: &str,
    before: Option<NodeId>,
) -> NodeId {
    let node = doc.create_element(tag);
    doc.set_attribute(node, "class", class);
    doc.insert_before(parent, node, before);
    node
}

/// Row `index`: a label cell, the value's cell and two more.
fn scroller_row(doc: &mut Document, parent: NodeId, index: usize, before: Option<NodeId>) -> Unit {
    let row = child_before(doc, parent, "div", "scroller", before);
    let label = element(doc, row, "div", "cell");
    text(doc, label, &format!("Row {index}"));
    let cell = element(doc, row, "div", "cell");
    let value = text(doc, cell, &value_text(index));
    for name in ["Owner", "Status"] {
        let cell = element(doc, row, "div", "cell");
        text(doc, cell, &format!("{name} {index}"));
    }
    Unit {
        element: row,
        value,
        inner: Some(row),
    }
}

/// The code block's lines after the value's: one wider than the block.
const CODE_LINES: [&str; 3] = [
    "fn main() {",
    "    let total = items.iter().map(|item| item.price * item.quantity).sum::<u64>(); // wider than the block",
    "}",
];

/// Message `index`: its prose, then its code block, whose first line
/// is the value.
fn message(doc: &mut Document, parent: NodeId, index: usize, before: Option<NodeId>) -> Unit {
    let unit = child_before(doc, parent, "div", "message", before);
    let prose = element(doc, unit, "p", "");
    text(doc, prose, &format!("Message {index}: the change below."));
    let code = element(doc, unit, "div", "code");
    let first = element(doc, code, "div", "line");
    let value = text(doc, first, &value_text(index));
    for line in CODE_LINES {
        let node = element(doc, code, "div", "line");
        text(doc, node, line);
    }
    Unit {
        element: unit,
        value,
        inner: Some(code),
    }
}
