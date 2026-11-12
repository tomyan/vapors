//! `html` given `display: none` by a class: no box at all. Here the
//! update paints nothing on the default white canvas, nothing is hit
//! and the a11y tree is empty; html shown again matches a full update.
//!
//! Checked against Chromium 155.0.8059.79 (Debian), 100x100 headless
//! screenshots, each page a `<!doctype html>` with inline styles:
//! `env -u DBUS_SESSION_BUS_ADDRESS chromium --headless --no-sandbox
//! --disable-gpu --window-size=100,100 --screenshot=out.png file://…/page.html`.
//! - html `display:none; background:red`, body `background:blue`: white.
//! - html `display:none` from a class, body `background:blue`: white.
//! - body `display:none; background:blue`: white.
//! - body `display:inline; background:blue`: blue.
//! - control, html `background:red`: red.
//!
//! The accessibility tree: the same Chromium with
//! `--remote-debugging-port`, `Accessibility.getFullAXTree` over CDP,
//! unignored nodes. html hidden by a class over a body of text and a
//! button gives only `RootWebArea` (the document, which this tree has no
//! node for); shown, it gives the text and the button.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::oracle::assert_matches_full;
use support::{document, element, text, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{CanvasFrom, Document, Viewport};
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
        painted.scene().to_scene().items().is_empty(),
        "{:?}",
        painted.scene().to_scene().items()
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
fn an_inline_body_still_gives_the_canvas_its_background() {
    // Given: body blue and inline, holding only text
    let mut doc = document();
    let body = doc.body();
    text(&mut doc, body, "hi");
    let sheet = "body { display: inline; margin: 0; background-color: blue; }";
    doc.add_stylesheet("app.css", sheet).expect("parses");

    // When
    let painted = doc.update(view()).expect("updates");

    // Then
    assert_eq!(painted.canvas(), Color::from_rgba8(0, 0, 255, 255));
    assert_eq!(painted.canvas_from(), Some(CanvasFrom::Body));
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
