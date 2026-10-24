//! Stacking (F2c S8): the order boxes are painted and hit in, by
//! z-index and stacking context, beyond what the `stacking` paint
//! fixture compares with Chromium.

mod support;

use support::{document, element};

#[test]
fn a_z_index_that_is_not_an_integer_is_refused_by_the_stylesheet() {
    // Given
    let mut doc = document();

    // When
    let error = doc
        .add_stylesheet("app.css", ".raised { position: relative; z-index: 1.5; }")
        .expect_err("refused");

    // Then
    let message = format!("{error:?}");
    assert!(
        message.contains("z-index") && message.contains("app.css"),
        "{message}"
    );
}

/// Two 40 px squares overlapping at (30, 30): `.low` first in the tree
/// with `low`, `.high` second with `high`.
fn overlapping(
    low: &str,
    high: &str,
) -> (
    vapors_elements::Document,
    vapors_dom::NodeId,
    vapors_dom::NodeId,
) {
    let mut doc = document();
    let body = doc.body();
    let first = element(&mut doc, body, "div", "low");
    let second = element(&mut doc, body, "div", "high");
    let css = format!(
        "body {{ margin: 0; }} div {{ position: absolute; width: 40px; height: 40px; }} \
         .low {{ left: 0; top: 0; {low} }} .high {{ left: 20px; top: 20px; {high} }}"
    );
    doc.add_stylesheet("app.css", &css).expect("parses");
    (doc, first, second)
}

#[test]
fn the_later_of_two_overlapping_positioned_boxes_is_hit() {
    // Given
    let (mut doc, _first, second) = overlapping("", "");

    // When
    let hit = doc
        .update(support::viewport(100.0, 100.0))
        .expect("updates")
        .hit([30.0, 30.0]);

    // Then
    assert_eq!(hit, Some(second));
}

#[test]
fn a_higher_z_index_is_hit_whatever_the_tree_order() {
    // Given: the first box has the higher z-index
    let (mut doc, first, _second) = overlapping("z-index: 2;", "z-index: 1;");

    // When
    let hit = doc
        .update(support::viewport(100.0, 100.0))
        .expect("updates")
        .hit([30.0, 30.0]);

    // Then
    assert_eq!(hit, Some(first));
}

#[test]
fn a_negative_z_index_paints_below_the_in_flow_boxes() {
    // Given: a negative-z square under an in-flow block it overlaps
    let mut doc = document();
    let body = doc.body();
    element(&mut doc, body, "div", "under");
    let block = element(&mut doc, body, "div", "block");
    doc.add_stylesheet(
        "app.css",
        "body { margin: 0; } .under { position: absolute; z-index: -1; width: 40px; height: 40px; } .block { height: 20px; }",
    )
    .expect("parses");

    // When
    let hit = doc
        .update(support::viewport(100.0, 100.0))
        .expect("updates")
        .hit([10.0, 10.0]);

    // Then: the block, painted later, is on top
    assert_eq!(hit, Some(block));
}

/// Two static 40 px squares in a `display` container, the second
/// pulled 20 px back over the first; the first has `first`.
fn static_overlap(
    display: &str,
    first: &str,
) -> (
    vapors_elements::Document,
    vapors_dom::NodeId,
    vapors_dom::NodeId,
) {
    let mut doc = document();
    let body = doc.body();
    let row = element(&mut doc, body, "div", "row");
    let one = element(&mut doc, row, "div", "one");
    let two = element(&mut doc, row, "div", "two");
    let css = format!(
        "body {{ margin: 0; }} .row {{ display: {display}; }} \
         .one, .two {{ width: 40px; height: 40px; }} .two {{ margin-top: -20px; margin-left: -20px; }} \
         .one {{ {first} }}"
    );
    doc.add_stylesheet("app.css", &css).expect("parses");
    (doc, one, two)
}

#[test]
fn a_z_index_on_a_static_flex_item_lifts_it() {
    // Given
    let (mut doc, one, _two) = static_overlap("flex", "z-index: 1;");

    // When
    let hit = doc
        .update(support::viewport(100.0, 100.0))
        .expect("updates")
        .hit([25.0, 10.0]);

    // Then
    assert_eq!(hit, Some(one));
}

#[test]
fn a_z_index_on_a_static_block_is_ignored() {
    // Given
    let (mut doc, _one, two) = static_overlap("block", "z-index: 1;");

    // When
    let hit = doc
        .update(support::viewport(100.0, 100.0))
        .expect("updates")
        .hit([10.0, 25.0]);

    // Then
    assert_eq!(hit, Some(two));
}
