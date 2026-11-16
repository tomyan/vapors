//! A hit test that lands on a scene's `gpu-surface` asks the scene's
//! source (`Dom::watch_hits`, docs/plans/css-for-3d.md C4): the anchor
//! it answers is the target, takes `:hover` and `:active` with its
//! ancestors, gets the pointer events, which bubble through the scene's
//! anchors to the scene anchor and its HTML ancestors, and gives the
//! cursor.

use crate::support;

use std::cell::RefCell;
use std::rc::Rc;
use support::pointer::record;
use support::scene_hits_page::{Answer, OFF, OVER_A, OVER_B, page};
use vapors_elements::{Cursor, Document};

#[test]
fn a_hit_on_the_surface_is_the_anchor_its_scene_answers() {
    // Given
    let page = page("");

    // Then
    assert_eq!(page.hit(OVER_A), Some(page.a));
    assert_eq!(page.hit(OVER_B), Some(page.b));
    assert_eq!(page.hit(OFF), None);
}

#[test]
fn the_scene_is_asked_with_the_point_from_the_content_box_and_its_size() {
    // Given: the content box starts at (22, 12) and is 200 x 100
    let page = page("");

    // When
    page.hit(OVER_A);

    // Then
    assert_eq!(*page.asks.borrow(), [([50.0, 40.0], [200.0, 100.0])]);
}

#[test]
fn where_the_scene_answers_nothing_the_surface_is_the_target() {
    // Given
    let page = page("");
    page.answer.set(Answer::Nothing);

    // Then
    assert_eq!(page.hit(OVER_A), Some(page.surface));
}

#[test]
fn an_anchor_with_pointer_events_none_or_hidden_is_not_taken() {
    for css in [".a { pointer-events: none; }", ".a { visibility: hidden; }"] {
        // Given
        let page = page(css);

        // Then: the source passes over a, so the surface is the target
        assert_eq!(page.hit(OVER_A), Some(page.surface), "{css}");
        assert_eq!(page.hit(OVER_B), Some(page.b), "{css}");
    }
}

#[test]
fn hovering_a_mesh_hovers_it_and_its_group() {
    // Given: a's colour shows its group's hover
    let page = page(".a { color: #000000; } .craft:hover .a { color: #ff0000; }");

    // When
    Document::pointer_moved(&page.doc, OVER_B);
    page.update();

    // Then: b is hit, and its group, a's parent, is hovered
    assert_eq!(page.a_red(), 1.0);

    // When: off the surface
    Document::pointer_moved(&page.doc, OFF);
    page.update();

    // Then
    assert_eq!(page.a_red(), 0.0);
}

#[test]
fn pressing_a_mesh_makes_it_active_until_release() {
    // Given
    let page = page(".a { color: #000000; } .a:active { color: #ff0000; }");

    // When
    Document::pointer_pressed(&page.doc, OVER_A);
    page.update();
    let pressed = page.a_red();
    Document::pointer_released(&page.doc, OVER_A);
    page.update();

    // Then
    assert_eq!((pressed, page.a_red()), (1.0, 0.0));
}

#[test]
fn a_click_on_a_mesh_reaches_its_anchor_and_bubbles_to_the_scene_and_body() {
    // Given
    let page = page("");
    let log = Rc::new(RefCell::new(Vec::new()));
    let body = page.doc.borrow().body();
    for (node, label) in [
        (page.a, "a"),
        (page.craft, "craft"),
        (page.scene, "scene"),
        (body, "body"),
        (page.surface, "surface"),
    ] {
        record(&page.doc, node, "click", label, &log);
    }

    // When
    Document::tapped(&page.doc, OVER_A);

    // Then: the surface, the scene anchor's sibling, is not on the path
    assert_eq!(
        *log.borrow(),
        ["a click", "craft click", "scene click", "body click"]
    );
}

#[test]
fn the_cursor_over_a_mesh_is_its_anchors() {
    // Given
    let page = page(".a { cursor: pointer; }");

    // Then
    let doc = page.doc.borrow();
    assert_eq!(doc.cursor(OVER_A), Cursor::Pointer);
    assert_eq!(doc.cursor(OVER_B), Cursor::Default);
}

#[test]
fn a_point_on_the_surfaces_border_is_the_surfaces_hit_and_does_not_ask() {
    // Given: the border runs from x 20 to 22
    let page = page("");

    // Then
    assert_eq!(page.hit([21.0, 52.0]), Some(page.surface));
    assert!(page.asks.borrow().is_empty());
}
