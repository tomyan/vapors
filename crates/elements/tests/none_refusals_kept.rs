//! Inside a `display: none` element, the refusals that do not depend on
//! whether an element is inline still refuse: what the author wrote is
//! unsupported whether or not it is shown.

mod support;

use support::element;
use support::hidden_page::{assert_refused, page, row, row_in};
use vapors_dom::Dom;

#[test]
fn a_hidden_img_naming_an_unregistered_image_is_refused() {
    // Given
    let (mut doc, row) = row("hidden");
    let img = element(&mut doc, row, "img", "");
    doc.set_attribute(img, "src", "missing.png");

    // Then
    assert_refused(&mut doc, "missing.png");
}

#[test]
fn a_hidden_element_with_a_rounded_clip_is_refused() {
    // Given: refused by the used-style reader, inline or not
    let (mut doc, row) = row("hidden");
    element(&mut doc, row, "div", "round");

    // Then
    assert_refused(&mut doc, "rounded clips");
}

#[test]
fn a_hidden_element_with_an_unsupported_cursor_is_refused() {
    // Given: refused when its var() is substituted, before the used style
    let (mut doc, row) = row("hidden");
    element(&mut doc, row, "div", "odd");

    // Then
    assert_refused(&mut doc, "cursor");
}

#[test]
fn an_inline_grandchild_of_a_hidden_row_inside_a_block_is_refused() {
    // Given: the span's block is the plain div, shown or not
    let mut doc = page("");
    let body = doc.body();
    let hidden = element(&mut doc, body, "div", "hidden");
    row_in(&mut doc, hidden, "");

    // Then
    assert_refused(&mut doc, "font-size");
}

#[test]
fn an_inline_element_under_a_hidden_html_is_refused() {
    // Given: html's child, body, is a block either way
    let mut doc = page("html { display: none; }");
    let body = doc.body();
    row_in(&mut doc, body, "");

    // Then
    assert_refused(&mut doc, "font-size");
}
