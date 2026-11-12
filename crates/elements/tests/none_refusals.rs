//! A child of a `display: none` element generates no box (CSS Display 3
//! section 2.5), and whether its parent will be a flex or grid container
//! once shown is unknown, so it is checked as blockified: the refusals
//! that depend on being inline wait until the parent is shown, and
//! showing it checks them again.

mod support;

use support::hidden_page::{assert_accepted, assert_accepted_at, assert_refused, page, row};
use support::{element, text};
use vapors_dom::Dom;

#[test]
fn hiding_a_flex_row_does_not_refuse_its_child_that_would_be_inline() {
    // Given
    let (mut doc, row) = row("row");
    assert_accepted(&mut doc);

    // When
    doc.set_attribute(row, "class", "row hidden");

    // Then
    assert_accepted(&mut doc);
}

#[test]
fn a_subtree_hidden_from_the_first_update_is_not_refused() {
    // Given
    let (mut doc, _) = row("hidden");

    // Then
    assert_accepted(&mut doc);
}

#[test]
fn showing_the_row_as_a_block_refuses_its_inline_child() {
    // Given
    let (mut doc, row) = row("hidden");
    assert_accepted(&mut doc);

    // When
    doc.set_attribute(row, "class", "");

    // Then
    assert_refused(&mut doc, "font-size");
}

#[test]
fn showing_the_row_as_a_flex_row_again_is_accepted() {
    // Given
    let (mut doc, row) = row("row hidden");
    assert_accepted(&mut doc);

    // When
    doc.set_attribute(row, "class", "row");

    // Then
    assert_accepted(&mut doc);
}

#[test]
fn a_style_refusal_inside_a_hidden_subtree_waits_until_it_is_shown() {
    // Given
    let (mut doc, row) = row("hidden");
    let padded = element(&mut doc, row, "span", "pad");
    text(&mut doc, padded, "P");
    assert_accepted(&mut doc);

    // When
    doc.set_attribute(row, "class", "");

    // Then
    assert_refused(&mut doc, "padding");
}

#[test]
fn a_block_in_the_inline_grandchild_of_a_hidden_row_waits_until_it_is_shown() {
    // Given: a span holding a div, blockified in a flex row
    let mut doc = page("");
    let body = doc.body();
    let row = element(&mut doc, body, "div", "row hidden");
    let span = element(&mut doc, row, "span", "");
    let div = element(&mut doc, span, "div", "");
    text(&mut doc, div, "D");
    assert_accepted(&mut doc);

    // When
    doc.set_attribute(row, "class", "");

    // Then
    assert_refused(&mut doc, "block-level inside an inline element");
}

#[test]
fn showing_the_row_by_its_style_attribute_checks_its_child_again() {
    // Given
    let (mut doc, row) = row("");
    doc.set_attribute(row, "style", "display: none");
    assert_accepted(&mut doc);

    // When
    doc.set_attribute(row, "style", "");

    // Then
    assert_refused(&mut doc, "font-size");
}

#[test]
fn showing_the_row_by_a_media_query_on_resize_checks_its_child_again() {
    // Given: hidden while the viewport is 50 px wide or less
    let (mut doc, _) = row("narrow");
    assert_accepted_at(&mut doc, 40.0);

    // Then
    assert_refused(&mut doc, "font-size");
}
