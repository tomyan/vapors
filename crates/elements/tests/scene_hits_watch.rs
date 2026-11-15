//! What `Dom::watch_hits` refuses, what a failing or wrong source does,
//! and `Dom::hits_moved` (docs/plans/css-for-3d.md C4): the hover under
//! a still pointer is tested again after the scene moved, and only then.

mod support;

use support::scene_hits_page::{Answer, OVER_A, OVER_SIDE, page};
use vapors_dom::Dom;
use vapors_elements::Document;

#[test]
fn a_source_on_anything_but_a_scene_anchor_is_refused() {
    // Given
    let page = page("");

    // When
    page.doc
        .borrow_mut()
        .watch_hits(page.a, "other", Box::new(|_| Ok(None)));

    // Then
    let refusals = page.refusals();
    assert!(
        refusals
            .iter()
            .any(|r| r.contains("watch_hits") && r.contains("scene")),
        "{refusals:?}"
    );
}

#[test]
fn a_second_source_on_a_scene_or_on_a_surface_is_refused() {
    // Given: the page's scene is watched on "view"
    let page = page("");
    let other = page.doc.borrow_mut().create_element("scene");
    let body = page.doc.borrow().body();
    page.doc.borrow_mut().append(body, other);

    // When
    page.doc
        .borrow_mut()
        .watch_hits(page.scene, "else", Box::new(|_| Ok(None)));
    page.doc
        .borrow_mut()
        .watch_hits(other, "view", Box::new(|_| Ok(None)));

    // Then
    let refusals = page.refusals();
    assert!(
        refusals.iter().any(|r| r.contains("already watched")),
        "{refusals:?}"
    );
    assert!(
        refusals.iter().any(|r| r.contains("\"view\"")),
        "{refusals:?}"
    );
}

#[test]
fn a_failing_source_targets_the_surface_and_refuses_the_next_update_once() {
    // Given
    let page = page("");
    page.answer.set(Answer::Fails);
    page.doc.borrow_mut().take_changed();

    // When
    let hit = page.hit(OVER_A);

    // Then: the update after it is due, and refused naming the cause
    assert_eq!(hit, Some(page.surface));
    assert!(page.doc.borrow_mut().take_changed());
    let refusals = page.refusals();
    assert!(
        refusals
            .iter()
            .any(|r| r.contains("scene") && r.contains("the camera is gone")),
        "{refusals:?}"
    );
    page.update();
}

#[test]
fn a_source_answering_a_node_outside_its_scene_is_refused() {
    // Given: it answers the surface, which is not in the scene
    let page = page("");
    page.answer.set(Answer::Node(page.surface));

    // When
    let hit = page.hit(OVER_A);

    // Then
    assert_eq!(hit, Some(page.surface));
    let refusals = page.refusals();
    assert!(
        refusals.iter().any(|r| r.contains("not in its scene")),
        "{refusals:?}"
    );
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
fn a_scene_moved_under_a_still_pointer_moves_the_hover_at_the_next_update() {
    // Given: the pointer rests over a; then the scene moves b there
    let page = page(".a { color: #000000; } .a:hover { color: #ff0000; }");
    Document::pointer_moved(&page.doc, OVER_A);
    page.update();
    page.answer.set(Answer::AllB);

    // When: an update without hits_moved keeps the hover
    page.update();
    let kept = (
        page.a_red(),
        page.doc.borrow().update_counts().hovers_tested,
    );

    // When: the scene says its hits moved
    page.doc.borrow_mut().hits_moved(page.scene);
    assert!(page.doc.borrow_mut().take_changed());
    page.update();
    let tested = page.doc.borrow().update_counts().hovers_tested;
    // the update that shows the moved hover restyles under the pointer,
    // and may test it once more as for HTML (`HitsChanged`); the one
    // after it is the still scene
    page.update();
    page.update();

    // Then
    assert_eq!(kept, (1.0, 0), "no hits_moved, no test");
    assert_eq!((tested, page.a_red()), (1, 0.0));
    assert_eq!(
        page.doc.borrow().update_counts().hovers_tested,
        0,
        "a still scene tests nothing"
    );
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
    assert_eq!(page.doc.borrow().update_counts().hovers_tested, 0);
    assert_eq!(page.asks.borrow().len(), asked);
}
