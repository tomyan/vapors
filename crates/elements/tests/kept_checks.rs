//! The style phase's checks are kept between updates and redone for the
//! elements changed since (IU3 PR 3a): each change that can add or drop
//! a refusal is seen by the next update.

mod support;

use support::svg_page::{PATH, icon, with_attributes};
use support::{document, element, refusals_at, text, viewport};
use vapors_dom::{Dom, Event, NodeId};
use vapors_elements::Document;
use vapors_scene::{RgbaImage, Size};

/// A document with a div, updated once.
fn updated() -> (Document, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let div = element(&mut doc, body, "div", "");
    doc.update(viewport(100.0, 100.0)).expect("updates");
    (doc, div)
}

fn listen_unfired(doc: &mut Document, node: NodeId) {
    doc.listen(node, "optionchoose", Box::new(|_: &mut Event| {}));
}

/// Asserts the update is refused with a message holding `says`. Debug
/// builds walk the tree on every update and panic when the kept checks
/// disagree, so the refusals are the ones a full check reports.
fn assert_refused(doc: &mut Document, says: &str) {
    let found = refusals_at(doc, viewport(100.0, 100.0));
    assert!(
        found.iter().any(|r| r.contains(says)),
        "{says} in {found:?}"
    );
}

fn assert_accepted(doc: &mut Document) {
    let updated = doc.update(viewport(100.0, 100.0));
    assert!(updated.is_ok(), "{:?}", updated.err());
}

/// A sized block img, class `pic`, updated once; `.round` rounds it.
fn updated_img() -> (Document, NodeId) {
    let (mut doc, div) = updated();
    let img = element(&mut doc, div, "img", "pic");
    let css = ".pic { display: block; width: 10px; height: 10px; } .round { border-radius: 3px; }";
    doc.add_stylesheet("app.css", css).expect("parses");
    assert_accepted(&mut doc);
    (doc, img)
}

#[test]
fn a_listener_added_after_an_update_for_an_event_never_fired_is_refused() {
    // Given
    let (mut doc, div) = updated();

    // When
    listen_unfired(&mut doc, div);

    // Then
    let found = refusals_at(&mut doc, viewport(100.0, 100.0));
    assert!(
        found.iter().any(|r| r.contains("onoptionchoose")),
        "{found:?}"
    );
}

#[test]
fn declaring_the_event_after_a_refusal_lets_the_update_through() {
    // Given
    let (mut doc, div) = updated();
    listen_unfired(&mut doc, div);
    refusals_at(&mut doc, viewport(100.0, 100.0));

    // When
    doc.declare_events(&["optionchoose"]);

    // Then
    assert!(doc.update(viewport(100.0, 100.0)).is_ok());
}

#[test]
fn removing_a_refused_element_lets_the_update_through() {
    // Given
    let (mut doc, _) = updated();
    let body = doc.body();
    let canvas = element(&mut doc, body, "canvas", "");
    refusals_at(&mut doc, viewport(100.0, 100.0));

    // When
    doc.remove(canvas);

    // Then
    assert!(doc.update(viewport(100.0, 100.0)).is_ok());
}

#[test]
fn an_attribute_set_after_an_update_that_the_tree_refuses_is_refused() {
    // Given
    let (mut doc, div) = updated();

    // When
    doc.set_attribute(div, "tabindex", "2");

    // Then
    let found = refusals_at(&mut doc, viewport(100.0, 100.0));
    assert!(found.iter().any(|r| r.contains("tabindex")), "{found:?}");
}

#[test]
fn fixing_an_attribute_the_tree_refused_lets_the_update_through() {
    // Given
    let (mut doc, div) = updated();
    doc.set_attribute(div, "tabindex", "2");
    assert_refused(&mut doc, "tabindex");

    // When
    doc.set_attribute(div, "tabindex", "0");

    // Then
    assert_accepted(&mut doc);
}

#[test]
fn a_class_that_adds_refused_css_is_refused_until_it_is_removed() {
    // Given
    let (mut doc, img) = updated_img();

    // When
    doc.set_attribute(img, "class", "pic round");
    assert_refused(&mut doc, "border-radius");
    doc.set_attribute(img, "class", "pic");

    // Then
    assert_accepted(&mut doc);
}

#[test]
fn right_to_left_text_set_after_an_update_is_refused_until_it_is_replaced() {
    // Given
    let (mut doc, div) = updated();
    let words = text(&mut doc, div, "hello");
    assert_accepted(&mut doc);

    // When
    doc.set_text(words, "\u{05E9}\u{05DC}\u{05D5}\u{05DD}");
    assert_refused(&mut doc, "right to left");
    doc.set_text(words, "hello");

    // Then
    assert_accepted(&mut doc);
}

/// The icon page updated once: its svg and its path.
fn updated_icon() -> (Document, NodeId, NodeId) {
    let (mut doc, svg) = icon("", &[], &[]);
    let path = doc.children(svg)[0];
    assert_accepted(&mut doc);
    (doc, svg, path)
}

#[test]
fn a_path_moved_out_of_its_svg_is_refused_until_it_is_moved_back() {
    // Given
    let (mut doc, svg, path) = updated_icon();
    let body = doc.body();

    // When
    doc.append(body, path);
    assert_refused(&mut doc, "is not inside an <svg>");
    doc.append(svg, path);

    // Then
    assert_accepted(&mut doc);
}

#[test]
fn an_svg_that_loses_its_last_path_is_refused_until_it_gains_one() {
    // Given
    let (mut doc, svg, path) = updated_icon();

    // When
    doc.remove(path);
    assert_refused(&mut doc, "holds no <path>");
    let path = element(&mut doc, svg, "path", "");
    with_attributes(&mut doc, path, PATH);

    // Then
    assert_accepted(&mut doc);
}

#[test]
fn an_img_naming_an_unregistered_image_is_refused_until_it_is_registered() {
    // Given
    let (mut doc, img) = updated_img();

    // When
    doc.set_attribute(img, "src", "late.png");
    assert_refused(&mut doc, "late.png");
    let rgba = [0, 0, 0, 255].repeat(4);
    let image = RgbaImage {
        size: Size {
            width: 2,
            height: 2,
        },
        rgba,
    };
    doc.register_image("late.png", image).expect("drawable");

    // Then
    assert_accepted(&mut doc);
}
