//! A box's clip chain across updates (flat-scene F3): a box kept from
//! the last update is measured again when a frame around it changes, so
//! a chain an ancestor's edit pushes past `MAX_CLIP_CHAIN` is refused,
//! and the document updates as a fresh one would after the refusal.

use crate::support;

use support::rows_page::{row, rows_viewport};
use support::{document, element, refusals_at, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, MAX_CLIP_CHAIN};

const CSS: &str = ".scroller { overflow: auto; height: 50px; } .leaf { height: 100px; } \
                   .outer { height: 60px; }";

/// An outer box that does not clip, `depth` scroll containers inside
/// it, and a leaf in the innermost. Returns the outer box.
fn nested_under(doc: &mut Document, parent: NodeId, depth: usize) -> NodeId {
    let outer = element(doc, parent, "div", "outer");
    let mut parent = outer;
    for _ in 0..depth {
        parent = element(doc, parent, "div", "scroller");
    }
    element(doc, parent, "div", "leaf");
    outer
}

fn at_the_limit() -> (Document, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let outer = nested_under(&mut doc, body, MAX_CLIP_CHAIN);
    doc.add_stylesheet("t.css", CSS).expect("parses");
    let longest = doc.update(viewport(640.0, 400.0)).expect("updates");
    assert_eq!(longest.longest_clip_chain(), MAX_CLIP_CHAIN);
    (doc, outer)
}

#[test]
fn an_ancestor_that_starts_to_scroll_pushes_a_kept_box_past_the_limit_and_is_refused() {
    // Given
    let (mut doc, outer) = at_the_limit();

    // When: the outer box scrolls; the leaf's own clip and frame are
    // unchanged, so it is not painted again
    doc.set_attribute(outer, "class", "outer scroller");
    let messages = refusals_at(&mut doc, viewport(640.0, 400.0));

    // Then
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert!(
        messages[0].starts_with("<div class=\"leaf\"> (box slot "),
        "{}",
        messages[0]
    );
    let links = format!("has {} links", MAX_CLIP_CHAIN + 1);
    assert!(messages[0].contains(&links), "{}", messages[0]);
}

#[test]
fn after_the_refusal_the_ancestor_edited_back_updates_at_the_limit() {
    // Given
    let (mut doc, outer) = at_the_limit();
    doc.set_attribute(outer, "class", "outer scroller");
    refusals_at(&mut doc, viewport(640.0, 400.0));

    // When: a debug build compares each update's paint with a fresh one
    doc.set_attribute(outer, "class", "outer");
    let updated = doc.update(viewport(640.0, 400.0));

    // Then
    let longest = updated.map(vapors_elements::Painted::longest_clip_chain);
    assert_eq!(longest.map_err(|e| format!("{e:?}")), Ok(MAX_CLIP_CHAIN));
}

/// Two hundred framed rows under body, which scrolls, the first holding
/// a chain at the limit (body's scroll frame is a link, and the row's
/// frame under body's clip another), updated once.
fn rows_with_a_chain_at_the_limit() -> (Document, Vec<NodeId>) {
    let mut doc = document();
    let rows: Vec<NodeId> = (0..200).map(|index| row(&mut doc, index).element).collect();
    nested_under(&mut doc, rows[0], MAX_CLIP_CHAIN - 2);
    let css = format!(
        "html {{ overflow: hidden; }} body {{ margin: 0; overflow: auto; height: 400px; }} .row {{ display: flex; height: 24px; gap: 8px; }} \
         .value {{ width: 80px; height: 20px; overflow: hidden; }} {CSS}"
    );
    doc.add_stylesheet("rows.css", &css).expect("parses");
    let longest = doc.update(rows_viewport()).expect("updates");
    assert_eq!(longest.longest_clip_chain(), MAX_CLIP_CHAIN);
    assert!(doc.update_counts().lanes_framed > 0, "the lane is framed");
    (doc, rows)
}

fn longest_after_update(doc: &mut Document) -> Result<usize, String> {
    doc.update(rows_viewport())
        .map(vapors_elements::Painted::longest_clip_chain)
        .map_err(|e| format!("{e:?}"))
}

#[test]
fn a_chain_at_the_limit_in_a_lane_row_updates_through_row_inserts_removals_and_scrolls() {
    // Given
    let (mut doc, rows) = rows_with_a_chain_at_the_limit();

    // When: a row inserted before the rest (every row starts elsewhere),
    // removed, then the page scrolled; a debug build compares each
    // update's paint, its refusals included, with a fresh one
    let inserted = row(&mut doc, 200).element;
    let body = doc.body();
    doc.insert_before(body, inserted, Some(rows[0]));
    let after_insert = longest_after_update(&mut doc);
    doc.remove(inserted);
    let after_remove = longest_after_update(&mut doc);
    assert!(
        doc.scroll_by([600.0, 300.0], [0.0, 30.0]),
        "the page scrolls"
    );
    let after_scroll = longest_after_update(&mut doc);

    // Then
    let at_limit = Ok(MAX_CLIP_CHAIN);
    assert_eq!(
        (after_insert, after_remove, after_scroll),
        (at_limit.clone(), at_limit.clone(), at_limit)
    );
}
