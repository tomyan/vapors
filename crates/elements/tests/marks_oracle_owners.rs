//! Owner resolution as boxes come and go (IU3 PR 1b): an element
//! hidden and shown, an inline element turned block and back, text
//! inside an inline span, a parent turned flex, and a calc restyled to
//! an equal computed style. Each matches a full update.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::oracle::assert_matches_full;
use support::{document, element, text, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};

const CSS: &str = ".hidden { display: none; } .block { display: block; width: 50px; } \
                   .flex { display: flex; } .calc { width: calc(50% + 10px); height: 10px; } \
                   .row { height: 40px; } .row:hover .calc { width: calc(50% + 10px); }";

/// A page: a block `div.item` with text, then a paragraph `div` holding
/// text, an inline `span` with text, and text.
struct Page {
    doc: Document,
    item: NodeId,
    paragraph: NodeId,
    span: NodeId,
    span_text: NodeId,
}

fn page() -> Page {
    let mut doc = document();
    let body = doc.body();
    let item = element(&mut doc, body, "div", "item");
    text(&mut doc, item, "Item");
    let paragraph = element(&mut doc, body, "div", "");
    text(&mut doc, paragraph, "Before ");
    let span = element(&mut doc, paragraph, "span", "");
    let span_text = text(&mut doc, span, "inside");
    text(&mut doc, paragraph, " after");
    doc.add_stylesheet("app.css", CSS).expect("parses");
    doc.update(view()).expect("updates");
    Page {
        doc,
        item,
        paragraph,
        span,
        span_text,
    }
}

fn view() -> Viewport {
    viewport(200.0, 200.0)
}

/// Sets `node`'s class to each of `classes` in turn, updating between.
fn classes_in_turn(doc: &mut Document, node: NodeId, classes: &[&str]) {
    for class in classes {
        doc.update(view()).expect("updates");
        doc.set_attribute(node, "class", class);
    }
}

fn assert_full(doc: Document, context: &str) {
    assert_matches_full(&Rc::new(RefCell::new(doc)), view(), context);
}

#[test]
fn a_block_hidden_then_shown_matches_a_full_update() {
    let Page { mut doc, item, .. } = page();
    classes_in_turn(&mut doc, item, &["hidden", ""]);
    assert_full(doc, "block, none, block");
}

#[test]
fn an_inline_span_made_block_then_inline_matches_a_full_update() {
    let Page { mut doc, span, .. } = page();
    classes_in_turn(&mut doc, span, &["block", ""]);
    assert_full(doc, "inline, block, inline");
}

#[test]
fn text_set_inside_an_inline_span_matches_a_full_update() {
    let Page {
        mut doc, span_text, ..
    } = page();
    doc.set_text(span_text, "inside, and longer");
    assert_full(doc, "text in a span");
}

#[test]
fn a_parent_made_flex_blockifies_its_inline_children_as_a_full_update() {
    let Page {
        mut doc, paragraph, ..
    } = page();
    doc.set_attribute(paragraph, "class", "flex");
    assert_full(doc, "parent made flex");
}

#[test]
fn a_calc_restyled_to_an_equal_computed_style_matches_a_full_update() {
    // Given: hovering the row restyles the cell to the same calc width
    let mut doc = document();
    let body = doc.body();
    let row = element(&mut doc, body, "div", "row");
    let cell = element(&mut doc, row, "div", "calc");
    text(&mut doc, cell, "Cell");
    doc.add_stylesheet("app.css", CSS).expect("parses");
    doc.update(view()).expect("updates");
    let doc = Rc::new(RefCell::new(doc));

    // When
    Document::pointer_moved(&doc, [20.0, 20.0]);

    // Then
    assert_matches_full(&doc, view(), "equal calc restyle");
}
