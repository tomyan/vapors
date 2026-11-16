//! Each scroll container's own thumb opacity (a touch app's fade): a
//! container's thumbs drawn at its own alpha, others' left as they are.

use crate::support;

use support::basics::{document, element};
use support::scroll_list::at;
use vapors_dom::NodeId;
use vapors_elements::Document;
use vapors_scene::{Color, Item};

const RED: Color = Color {
    r: 1.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};

/// Two 100 x 50 lists side by side, each with 160 px of content and red
/// thumbs.
fn two_lists() -> (Document, NodeId, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let lists = [0, 1].map(|_| {
        let list = element(&mut doc, body, "div", "list");
        for _ in 0..4 {
            element(&mut doc, list, "div", "row");
        }
        list
    });
    let css = "body { margin: 0; display: flex; } .row { height: 40px; } \
               .list { width: 100px; height: 50px; overflow: auto; scrollbar-color: red transparent; }";
    doc.add_stylesheet("app.css", css).expect("parses");
    doc.set_thumbs_shown(false);
    doc.update(at(1.0)).expect("updates");
    (doc, lists[0], lists[1])
}

/// The alphas of the red thumbs the update paints, left to right.
fn thumb_alphas(doc: &mut Document) -> Vec<f32> {
    let painted = doc.update(at(1.0)).expect("updates");
    let mut thumbs: Vec<(f32, f32)> = painted
        .scene()
        .to_scene()
        .items()
        .iter()
        .filter_map(|item| match item {
            Item::Quad(quad) if (quad.background.r, quad.background.g) == (RED.r, RED.g) => {
                Some((quad.rect.left, quad.background.a))
            }
            _ => None,
        })
        .collect();
    thumbs.sort_by(|a, b| a.0.total_cmp(&b.0));
    thumbs.into_iter().map(|(_, alpha)| alpha).collect()
}

#[test]
fn a_containers_own_opacity_shows_its_thumbs_and_no_others() {
    // Given two lists, thumbs hidden
    let (mut doc, left, _) = two_lists();
    assert_eq!(thumb_alphas(&mut doc), Vec::<f32>::new());

    // When the left list's thumbs are shown, then fade to half
    doc.set_thumb_opacity_of(left, 1.0);
    let shown = thumb_alphas(&mut doc);
    doc.set_thumb_opacity_of(left, 0.5);
    let half = thumb_alphas(&mut doc);

    // Then: only the left list's thumb is drawn, at its alpha
    assert_eq!(shown, vec![1.0]);
    assert_eq!(half, vec![128.0 / 255.0]);
}

#[test]
fn every_containers_opacity_sets_a_floor_under_each_own() {
    // Given the right list shown on its own
    let (mut doc, _, right) = two_lists();
    doc.set_thumb_opacity_of(right, 1.0);

    // When every container's thumbs are shown (a mouse)
    doc.set_thumbs_shown(true);

    // Then both are drawn, fully
    assert_eq!(thumb_alphas(&mut doc), vec![1.0, 1.0]);
}

#[test]
fn an_element_that_does_not_scroll_takes_no_opacity() {
    // Given
    let (mut doc, _, _) = two_lists();
    let body = doc.body();
    doc.take_changed();

    // When
    doc.set_thumb_opacity_of(body, 1.0);

    // Then
    assert!(!doc.take_changed());
    assert_eq!(thumb_alphas(&mut doc), Vec::<f32>::new());
}

#[test]
fn a_thumbs_opacity_changes_nothing_the_a11y_mirror_reads() {
    // Given two lists, the mirror up to date
    let (mut doc, left, _) = two_lists();
    doc.take_ax_update().expect("takes");

    // When a fade step changes the left list's thumbs
    doc.set_thumb_opacity_of(left, 0.5);
    doc.update(at(1.0)).expect("updates");

    // Then the mirror reads nothing again: a thumb is not in the a11y
    // tree
    assert_eq!(doc.take_ax_update().expect("takes"), None);
    assert_eq!(doc.ax_take_counts().nodes_read, 0);
}
