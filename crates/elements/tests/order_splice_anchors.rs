//! Where an inserted row's steps go (`order_splice_anchors.rs`, section
//! (c) of `docs/plans/splice.md`): after the last in-flow box of the
//! previous sibling, skipping positioned boxes, or before the next
//! in-flow sibling; declined when the previous sibling holds a scroll
//! container or the owner keeps only out-of-flow children. Each also
//! matches a full update through the oracle.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::basics::{document, element, text, viewport};
use support::oracle::assert_matches_full;
use support::rows_page::row_before;
use support::splice_page::{
    SPLICED, assert_declined, inserted, matches_full, order, page, updated,
};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, SpliceDecline};

/// An owner `ctx` (a z-indexed context, 100px tall) holding `children`
/// described as (class, css); the sheet also sizes `.item`.
fn context(css: &str, children: &[&str]) -> (Rc<RefCell<Document>>, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let ctx = element(&mut doc, body, "div", "ctx");
    for class in children {
        let child = element(&mut doc, ctx, "div", class);
        text(&mut doc, child, class);
    }
    let sheet = format!(
        "body {{ margin: 0; }} .ctx {{ position: relative; z-index: 1; }} \
         .item {{ height: 30px; background-color: #ccf; outline: 1px solid #000; }} {css}"
    );
    doc.add_stylesheet("anchors.css", &sheet).expect("parses");
    let doc = Rc::new(RefCell::new(doc));
    doc.borrow_mut()
        .update(viewport(640.0, 400.0))
        .expect("updates");
    (doc, ctx)
}

fn appended_item(doc: &Rc<RefCell<Document>>, parent: NodeId) {
    let mut doc = doc.borrow_mut();
    let row = doc.create_element("div");
    doc.set_attribute(row, "class", "item");
    doc.insert_before(parent, row, None);
    let words = doc.create_text("new");
    doc.insert_before(row, words, None);
}

fn updated_counts(doc: &Rc<RefCell<Document>>) -> vapors_elements::UpdateCounts {
    doc.borrow_mut()
        .update(viewport(640.0, 400.0))
        .expect("updates");
    doc.borrow().update_counts()
}

#[test]
fn an_insert_after_a_sibling_holding_a_scroll_container_is_declined_and_walked() {
    // Given: the last child holds a scroller in flow
    let (doc, ctx) = context(
        ".scroller { height: 60px; overflow: scroll; }",
        &["item", "item scroller"],
    );

    // When
    appended_item(&doc, ctx);
    let counts = updated_counts(&doc);

    // Then
    assert_declined(counts, SpliceDecline::ScrollInPreviousSibling);
    assert_matches_full(&doc, viewport(640.0, 400.0), "after scroller");
}

#[test]
fn an_insert_into_an_owner_whose_only_kept_child_is_out_of_flow_is_declined_and_walked() {
    // Given: the context's only child is a negative z-index layer
    let (doc, ctx) = context(
        ".under { position: absolute; z-index: -1; width: 50px; height: 50px; }",
        &["under"],
    );

    // When
    appended_item(&doc, ctx);
    let counts = updated_counts(&doc);

    // Then: spliced, the row would land before the negative layer
    assert_declined(counts, SpliceDecline::OnlyOutOfFlowKept);
    assert_matches_full(&doc, viewport(640.0, 400.0), "only out of flow");
}

#[test]
fn an_append_after_a_row_whose_last_cell_is_positioned_goes_after_its_last_in_flow_box() {
    // Given
    let page = page(4);
    page.doc
        .borrow_mut()
        .add_stylesheet("tail.css", ".row.tail > button { position: relative; }")
        .expect("parses");
    inserted(&page, None, "tail");
    updated(&page);

    // When
    inserted(&page, None, "");
    let counts = updated(&page);

    // Then
    assert_eq!(order(counts), SPLICED);
    matches_full(&page, "positioned last cell");
}

#[test]
fn an_insert_before_a_positioned_row_goes_before_the_next_in_flow_row() {
    // Given: a positioned row between rows 1 and 2
    let page = page(4);
    let positioned = {
        let mut doc = page.doc.borrow_mut();
        let row = row_before(&mut doc, 60, Some(page.rows[2].element)).element;
        doc.set_attribute(row, "class", "row positioned");
        row
    };
    updated(&page);

    // When
    row_before(&mut page.doc.borrow_mut(), 61, Some(positioned));
    let counts = updated(&page);

    // Then
    assert_eq!(order(counts), SPLICED);
    matches_full(&page, "before positioned");
}
