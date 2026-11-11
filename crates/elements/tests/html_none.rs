//! `html` given `display: none` by a class: no box at all. Chromium
//! (155, headless screenshot) then paints only the default white
//! canvas, even when html and body set backgrounds, and its
//! accessibility tree holds no node. The same white canvas when only
//! body is hidden. Here the update paints nothing on
//! white, nothing is hit and the a11y tree is empty; html shown again
//! matches a full update.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::oracle::assert_matches_full;
use support::{document, element, text, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};
use vapors_scene::Color;

const CSS: &str = "html { background-color: red; } body { background-color: blue; } \
                   .hidden { display: none; } button { display: block; width: 50px; height: 20px; }";

/// A body holding text and a button, updated once with html shown.
fn page() -> Document {
    let mut doc = document();
    let body = doc.body();
    text(&mut doc, body, "Before");
    let button = element(&mut doc, body, "button", "");
    text(&mut doc, button, "Go");
    doc.add_stylesheet("app.css", CSS).expect("parses");
    doc.update(view()).expect("updates");
    doc
}

fn view() -> Viewport {
    viewport(200.0, 100.0)
}

fn hide_root(doc: &mut Document) -> NodeId {
    let root = doc.root();
    doc.set_attribute(root, "class", "hidden");
    root
}

#[test]
fn html_hidden_paints_nothing_on_the_default_white_canvas() {
    // Given
    let mut doc = page();
    hide_root(&mut doc);

    // When
    let painted = doc.update(view()).expect("updates");

    // Then
    assert!(
        painted.scene().items().is_empty(),
        "{:?}",
        painted.scene().items()
    );
    assert_eq!(painted.canvas(), Color::from_rgba8(255, 255, 255, 255));
    assert_eq!(painted.canvas_from(), None);
}

#[test]
fn body_hidden_gives_the_canvas_no_colour() {
    // Given: html transparent, body blue and hidden
    let mut doc = page();
    let (root, body) = (doc.root(), doc.body());
    doc.set_attribute(root, "style", "background-color: transparent");
    doc.set_attribute(body, "class", "hidden");

    // When
    let painted = doc.update(view()).expect("updates");

    // Then
    assert_eq!(painted.canvas(), Color::from_rgba8(255, 255, 255, 255));
    assert_eq!(painted.canvas_from(), None);
}

#[test]
fn html_hidden_hits_nothing_and_has_no_box() {
    // Given
    let mut doc = page();
    let root = hide_root(&mut doc);

    // When
    let painted = doc.update(view()).expect("updates");

    // Then
    assert_eq!(painted.hit([10.0, 10.0]), None);
    assert_eq!(painted.border_box(root), None);
}

#[test]
fn html_hidden_takes_pointer_and_wheel_input_with_nothing_under_it() {
    // Given
    let mut doc = page();
    hide_root(&mut doc);
    doc.update(view()).expect("updates");
    let doc = Rc::new(RefCell::new(doc));

    // When
    Document::pointer_moved(&doc, [10.0, 10.0]);
    Document::pointer_pressed(&doc, [10.0, 10.0]);
    Document::pointer_released(&doc, [10.0, 10.0]);
    let scrolled = doc.borrow_mut().scroll_by([10.0, 10.0], [0.0, 40.0]);
    doc.borrow_mut().update(view()).expect("updates");

    // Then
    assert!(!scrolled, "nothing to scroll");
    assert_eq!(doc.borrow().active_element(), None);
}

#[test]
fn html_hidden_has_an_empty_a11y_tree() {
    // Given
    let mut doc = page();
    hide_root(&mut doc);
    doc.update(view()).expect("updates");

    // When
    let tree = doc.ax_tree().expect("an updated document");

    // Then
    assert!(tree.is_empty(), "{tree:?}");
}

#[test]
fn html_hidden_matches_a_full_update() {
    // Given
    let mut doc = page();
    hide_root(&mut doc);

    // Then
    assert_matches_full(&Rc::new(RefCell::new(doc)), view(), "html none");
}

#[test]
fn html_hidden_then_shown_matches_a_full_update() {
    // Given
    let mut doc = page();
    let root = hide_root(&mut doc);
    doc.update(view()).expect("updates");

    // When
    doc.set_attribute(root, "class", "");

    // Then
    assert_matches_full(&Rc::new(RefCell::new(doc)), view(), "html none, then shown");
}
