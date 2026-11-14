//! The layout pages: containers whose children's places depend on
//! each other (flex wrapping, grid tracks, inline text wrapping into
//! lines), which the planned layout lane declines and Taffy lays out
//! (docs/plans/splice-layout.md, section 4).

use vapors_dom::NodeId;
use vapors_elements::Document;

use crate::off_path_page::{Shape, Unit, body, child_before, value_text};
use crate::rows::basics::{element, text};

/// A wrapping flex row of 140 px cards: four to a line in 640 px.
pub const WRAPPED_CARDS: Shape = Shape {
    css: ".cards { display: flex; flex-wrap: wrap; gap: 8px; padding: 8px; } \
          .card { width: 140px; height: 56px; padding: 4px; background-color: #f0f0f4; }",
    container: cards,
    unit: card,
};

/// A grid of seven 1fr columns, a cell per unit.
pub const GRID_CELLS: Shape = Shape {
    css: ".grid { display: grid; grid-template-columns: repeat(7, 1fr); gap: 4px; } \
          .cell { height: 20px; }",
    container: grid,
    unit: cell,
};

/// Paragraphs of one to five sentences on each side of a bold span and
/// the value in a coloured span, wrapping into two to nine 21 px lines
/// at 640 px.
pub const MIXED_TEXT: Shape = Shape {
    css: ".para { margin: 0 0 8px 0; } .em { font-weight: 700; } .value { color: #36c; }",
    container: body,
    unit: paragraph,
};

fn cards(doc: &mut Document) -> NodeId {
    let body = doc.body();
    element(doc, body, "div", "cards")
}

fn grid(doc: &mut Document) -> NodeId {
    let body = doc.body();
    element(doc, body, "div", "grid")
}

/// Card `index`: its title, then its value.
fn card(doc: &mut Document, parent: NodeId, index: usize, before: Option<NodeId>) -> Unit {
    let card = child_before(doc, parent, "div", "card", before);
    let title = element(doc, card, "div", "");
    text(doc, title, &format!("Card {index}"));
    let line = element(doc, card, "div", "");
    let value = text(doc, line, &value_text(index));
    Unit {
        element: card,
        value,
        inner: None,
    }
}

/// Cell `index`, holding its value.
fn cell(doc: &mut Document, parent: NodeId, index: usize, before: Option<NodeId>) -> Unit {
    let cell = child_before(doc, parent, "div", "cell", before);
    let value = text(doc, cell, &value_text(index));
    Unit {
        element: cell,
        value,
        inner: None,
    }
}

const SENTENCE: &str = "The retoucher lifts the shadows by the window and keeps the sky. ";

/// The sentences paragraph `index` has on each side of its spans: one to
/// five, by `index % 5`. Every size is a multiple of ten, so the edited
/// paragraph (`count / 2`) and the inserted one (`count`) have four at
/// every size.
#[must_use]
pub fn sentences(index: usize) -> usize {
    1 + (index + 3) % 5
}

/// Paragraph `index`: its [`sentences`] before and after a bold span
/// and the value's span.
fn paragraph(doc: &mut Document, parent: NodeId, index: usize, before: Option<NodeId>) -> Unit {
    let para = child_before(doc, parent, "p", "para", before);
    let sentences = SENTENCE.repeat(sentences(index));
    text(doc, para, &format!("{index}. {sentences}"));
    let em = element(doc, para, "span", "em");
    text(doc, em, "Curves");
    text(doc, para, " set to ");
    let span = element(doc, para, "span", "value");
    let value = text(doc, span, &value_text(index));
    text(doc, para, &format!(" {sentences}"));
    Unit {
        element: para,
        value,
        inner: None,
    }
}

#[cfg(test)]
#[path = "off_path_layouts_tests.rs"]
mod tests;
