//! Scroll containers' thumbs (F2c S9): an overlay thumb in the scene,
//! styled by `scrollbar-color` and `scrollbar-width`, placed by the
//! offset, taking no layout space.

mod support;

use support::refusals;
use support::scroll_list::{List, at, list};
use vapors_scene::{Color, Item, Rect};

/// The rects of the scene's quads painted `color`.
fn quads_of(list: &mut List, color: Color) -> Vec<Rect> {
    let painted = list.doc.update(at(1.0)).expect("updates");
    painted
        .scene()
        .items()
        .iter()
        .filter_map(|item| match item {
            Item::Quad(quad) if quad.background == color => Some(quad.rect),
            _ => None,
        })
        .collect()
}

const RED: Color = Color {
    r: 1.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};

#[test]
fn a_scroll_container_draws_a_thumb_in_the_colour_asked_for() {
    // Given: the list is 100 x 50 at the origin, 160 px of content
    let mut list = list("overflow: auto; scrollbar-color: red transparent;");

    // When
    let thumbs = quads_of(&mut list, RED);

    // Then: one vertical thumb, 8 px wide, 2 px in from the right edge
    assert_eq!(thumbs.len(), 1, "{thumbs:?}");
    assert_eq!((thumbs[0].left, thumbs[0].right), (90.0, 98.0));
    assert_eq!(thumbs[0].top, 2.0);
}

#[test]
fn the_thumb_moves_with_the_offset() {
    // Given
    let mut list = list("overflow: auto; scrollbar-color: red transparent;");
    let before = quads_of(&mut list, RED)[0];

    // When: to the end
    list.doc.scroll_by([10.0, 10.0], [0.0, 1000.0]);
    let after = quads_of(&mut list, RED)[0];

    // Then: its bottom is 2 px above the list's
    assert!(after.top > before.top);
    assert_eq!(after.bottom, 48.0);
}

#[test]
fn thin_draws_a_narrower_thumb_and_none_draws_none() {
    // Given
    let mut thin = list("overflow: auto; scrollbar-color: red transparent; scrollbar-width: thin;");
    let mut none = list("overflow: auto; scrollbar-color: red transparent; scrollbar-width: none;");

    // When / Then
    let thumb = quads_of(&mut thin, RED)[0];
    assert_eq!(thumb.right - thumb.left, 4.0);
    assert_eq!(quads_of(&mut none, RED), []);
}

#[test]
fn content_that_fits_draws_no_thumb() {
    // Given: a list taller than its rows
    let mut list = list("overflow: auto; height: 400px; scrollbar-color: red transparent;");

    // When / Then
    assert_eq!(quads_of(&mut list, RED), []);
}

#[test]
fn a_track_colour_is_refused() {
    // Given
    let mut list = list("overflow: auto; scrollbar-color: red white;");

    // When
    let errors = refusals(&mut list.doc);

    // Then
    assert!(
        errors
            .iter()
            .any(|e| e.contains("scrollbar-color") && e.contains("track")),
        "{errors:?}"
    );
}

#[test]
fn hidden_thumbs_are_not_drawn_and_showing_them_again_redraws() {
    // Given
    let mut list = list("overflow: auto; scrollbar-color: red transparent;");
    list.doc.update(at(1.0)).expect("updates");
    list.doc.take_changed();

    // When / Then: hidden
    list.doc.set_thumbs_shown(false);
    assert!(list.doc.take_changed());
    assert_eq!(quads_of(&mut list, RED), []);

    // When / Then: hiding again changes nothing; showing redraws them
    list.doc.set_thumbs_shown(false);
    assert!(!list.doc.take_changed());
    list.doc.set_thumbs_shown(true);
    assert!(list.doc.take_changed());
    assert_eq!(quads_of(&mut list, RED).len(), 1);
}
