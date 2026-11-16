//! When the hover under a resting pointer is tested again for a scene
//! (docs/plans/css-for-3d.md C4): after `Dom::hits_moved`, a removed
//! scene, or an anchor that stops taking the pointer, while the pointer
//! is over that scene; and only then. The scene's anchors sit in a
//! holder away from the pointer, so no box under it is rebuilt.

mod support;

use support::pointer::quad_colours;
use support::scene_hits_page::{Answer, OVER_A, OVER_B, OVER_SIDE, Page, page};
use vapors_dom::Dom;
use vapors_elements::Document;

const HOVER_A: &str = ".a { color: #000000; } .a:hover { color: #ff0000; }";

/// The page with `css`, the pointer resting over `a`, shown.
fn resting_on_a(css: &str) -> Page {
    let page = page(css);
    Document::pointer_moved(&page.doc, OVER_A);
    page.update();
    page
}

fn hovers_tested(page: &Page) -> usize {
    page.doc.borrow().update_counts().hovers_tested
}

#[test]
fn a_removed_scene_is_not_asked() {
    // Given
    let page = page("");

    // When
    page.doc.borrow_mut().remove(page.scene);
    page.update();

    // Then
    assert_eq!(page.hit(OVER_A), Some(page.surface));
    assert!(page.asks.borrow().is_empty());
}

#[test]
fn a_scene_removed_under_a_resting_pointer_gives_the_hover_to_the_surface() {
    // Given: the surface shows its hover in its background
    let page = resting_on_a("gpu-surface:hover { background-color: #ff0000; }");

    // When
    page.doc.borrow_mut().remove(page.scene);
    page.update();
    page.update();

    // Then
    assert!(
        quad_colours(&page.doc).contains(&[255, 0, 0, 255]),
        "{:?}",
        quad_colours(&page.doc)
    );
}

#[test]
fn a_scene_whose_hits_move_without_hits_moved_keeps_the_hover() {
    // Given
    let page = resting_on_a(HOVER_A);
    page.answer.set(Answer::AllB);

    // When
    page.update();

    // Then
    assert_eq!((page.a_red(), hovers_tested(&page)), (1.0, 0));
}

#[test]
fn hits_moved_under_a_resting_pointer_moves_the_hover_at_the_next_update() {
    // Given
    let page = resting_on_a(HOVER_A);
    page.answer.set(Answer::AllB);

    // When
    page.doc.borrow_mut().hits_moved(page.scene);
    assert!(page.doc.borrow_mut().take_changed());
    page.update();
    let tested = hovers_tested(&page);
    page.update();

    // Then
    assert_eq!((tested, page.a_red()), (1, 0.0));
}

#[test]
fn a_still_scene_after_a_move_tests_nothing() {
    // Given: hits_moved, shown by the updates after it
    let page = resting_on_a(HOVER_A);
    page.doc.borrow_mut().hits_moved(page.scene);
    page.update();
    page.update();

    // When
    page.update();

    // Then
    assert_eq!(hovers_tested(&page), 0);
}

#[test]
fn hits_moved_on_a_node_whose_hits_are_not_watched_is_refused() {
    // Given
    let page = page("");

    // When
    page.doc.borrow_mut().hits_moved(page.craft);

    // Then
    let refusals = page.refusals();
    assert!(
        refusals
            .iter()
            .any(|r| r.contains("hits_moved") && r.contains("not watched")),
        "{refusals:?}"
    );
}

#[test]
fn a_scene_moved_while_the_pointer_is_elsewhere_tests_nothing() {
    // Given
    let page = page("");
    Document::pointer_moved(&page.doc, OVER_SIDE);
    page.update();
    assert_eq!(
        page.hit(OVER_SIDE),
        Some(page.side),
        "resting on an HTML element"
    );
    page.doc.borrow_mut().take_changed();
    let asked = page.asks.borrow().len();

    // When
    page.doc.borrow_mut().hits_moved(page.scene);

    // Then
    assert!(!page.doc.borrow_mut().take_changed());
    page.update();
    assert_eq!(hovers_tested(&page), 0);
    assert_eq!(page.asks.borrow().len(), asked);
}

#[test]
fn a_hovered_anchor_that_stops_taking_the_pointer_loses_the_hover_at_the_next_update() {
    for rule in [
        ".off { pointer-events: none; }",
        ".off { visibility: hidden; }",
    ] {
        // Given
        let page = resting_on_a(&format!("{HOVER_A} {rule}"));

        // When
        page.doc
            .borrow_mut()
            .set_attribute(page.a, "class", "a off");
        page.update();
        page.update();

        // Then: the scene passes over a, so the hover left it
        assert_eq!(page.a_red(), 0.0, "{rule}");
    }
}

#[test]
fn a_hovered_anchor_removed_from_the_scene_loses_the_hover_at_the_next_update() {
    // Given: the pointer rests over b; the group's hover shows on a
    let page = page(".a { color: #000000; } .craft:hover .a { color: #ff0000; }");
    Document::pointer_moved(&page.doc, OVER_B);
    page.update();
    let hovered = page.a_red();

    // When: b is removed, and the scene answers nothing there now
    page.answer.set(Answer::Nothing);
    page.doc.borrow_mut().remove(page.b);
    page.update();
    page.update();

    // Then
    assert_eq!((hovered, page.a_red()), (1.0, 0.0));
}
