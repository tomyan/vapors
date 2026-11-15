//! What `Dom::watch_hits` refuses, and what a failing or mistaken
//! source does (docs/plans/css-for-3d.md C4): the surface is the target
//! for that hit, and the next update is refused, once per message.

mod support;

use support::scene_hits_page::{Answer, OVER_A, page};
use vapors_dom::Dom;
use vapors_elements::Document;

fn refused_with(page: &support::scene_hits_page::Page, text: &str) -> usize {
    let refusals = page.refusals();
    let found = refusals.iter().filter(|r| r.contains(text)).count();
    assert!(found > 0, "no refusal containing {text:?}: {refusals:?}");
    found
}

#[test]
fn a_source_on_anything_but_a_scene_anchor_is_refused() {
    // Given
    let page = page("");

    // When
    page.doc
        .borrow_mut()
        .watch_hits(page.a, "other", Box::new(|_| Ok(None)));

    // Then
    refused_with(&page, "only a 3D `scene` anchor");
}

#[test]
fn a_second_source_on_a_scene_is_refused() {
    // Given: the page's scene is watched on "view"
    let page = page("");

    // When
    page.doc
        .borrow_mut()
        .watch_hits(page.scene, "else", Box::new(|_| Ok(None)));

    // Then
    refused_with(&page, "already watched");
}

#[test]
fn a_second_scene_on_a_surface_is_refused() {
    // Given
    let page = page("");
    let other = page.doc.borrow_mut().create_element("scene");
    let body = page.doc.borrow().body();
    page.doc.borrow_mut().append(body, other);

    // When
    page.doc
        .borrow_mut()
        .watch_hits(other, "view", Box::new(|_| Ok(None)));

    // Then
    refused_with(&page, "\"view\" already has a scene");
}

#[test]
fn a_failing_source_targets_the_surface_and_makes_the_next_update_due_and_refused() {
    // Given
    let page = page("");
    page.answer.set(Answer::Fails);
    page.doc.borrow_mut().take_changed();

    // When
    let hit = page.hit(OVER_A);

    // Then
    assert_eq!(hit, Some(page.surface));
    assert!(page.doc.borrow_mut().take_changed());
    refused_with(&page, "the camera is gone");
    page.update();
}

#[test]
fn a_refusal_met_many_times_before_an_update_is_refused_once() {
    // Given
    let page = page("");
    page.answer.set(Answer::Fails);

    // When
    page.hit(OVER_A);
    page.hit(OVER_A);
    let _ = page.doc.borrow().cursor(OVER_A);

    // Then
    assert_eq!(refused_with(&page, "the camera is gone"), 1);
}

#[test]
fn a_refusal_met_by_the_hover_test_inside_an_update_refuses_the_update_after() {
    // Given: the pointer rests over the scene, whose source then fails
    let page = page("");
    Document::pointer_moved(&page.doc, OVER_A);
    page.update();
    page.answer.set(Answer::Fails);

    // When: the update tests the hover (hits_moved) and meets the error
    page.doc.borrow_mut().hits_moved(page.scene);
    page.update();

    // Then: that update succeeded; the next is refused
    refused_with(&page, "the camera is gone");
}

#[test]
fn an_answer_outside_the_scene_is_refused() {
    // Given: it answers the surface, which is not in the scene
    let page = page("");
    page.answer.set(Answer::Node(page.surface));

    // Then
    assert_eq!(page.hit(OVER_A), Some(page.surface));
    refused_with(&page, "not an anchor in its scene");
}

#[test]
fn an_answer_that_does_not_take_the_pointer_is_refused() {
    // Given: a source that ignores takes, answering a with pointer-events: none
    let page = page(".a { pointer-events: none; }");
    page.answer.set(Answer::Node(page.a));

    // Then
    assert_eq!(page.hit(OVER_A), Some(page.surface));
    refused_with(&page, "does not take the pointer");
}

#[test]
fn asking_about_another_scenes_anchor_is_refused() {
    // Given: a mesh in a scene this source does not answer for
    let page = page("");
    let body = page.doc.borrow().body();
    let other = page.doc.borrow_mut().create_element("scene");
    let mesh = page.doc.borrow_mut().create_element("mesh");
    page.doc.borrow_mut().append(body, other);
    page.doc.borrow_mut().append(other, mesh);
    page.update();
    page.answer.set(Answer::AsksAbout(mesh, None));

    // When
    page.hit(OVER_A);

    // Then
    refused_with(&page, "not an anchor in its scene");
}

#[test]
fn asking_about_a_node_in_the_scene_that_is_no_anchor_is_refused() {
    // Given: a text node under the group
    let page = page("");
    let text = page.doc.borrow_mut().create_text("x");
    page.doc.borrow_mut().append(page.craft, text);
    page.answer.set(Answer::AsksAbout(text, None));

    // When
    page.hit(OVER_A);

    // Then
    refused_with(&page, "not an anchor in its scene");
}

#[test]
fn an_anchor_added_since_the_update_leaves_the_surface_the_target_until_the_next() {
    // Given: a mesh added to the scene and not yet styled, under the point
    let page = page("");
    let c = page.doc.borrow_mut().create_element("mesh");
    page.doc.borrow_mut().append(page.craft, c);
    page.answer.set(Answer::Node(c));

    // When
    let before = page.hit(OVER_A);
    page.update();

    // Then: no refusal, and once styled it is the target
    assert_eq!(before, Some(page.surface));
    assert_eq!(page.hit(OVER_A), Some(c));
}

#[test]
fn an_answer_reached_past_an_anchor_added_since_the_update_is_not_trusted() {
    // Given: a mesh added in front, not yet styled; the source asks
    // about it, is told it does not take the pointer, and answers a
    let page = page("");
    let c = page.doc.borrow_mut().create_element("mesh");
    page.doc.borrow_mut().append(page.craft, c);
    page.answer.set(Answer::AsksAbout(c, Some(page.a)));

    // When
    let before = page.hit(OVER_A);
    page.update();

    // Then: the surface until c is styled, then the answer stands
    assert_eq!(before, Some(page.surface));
    assert_eq!(page.hit(OVER_A), Some(page.a));
}
