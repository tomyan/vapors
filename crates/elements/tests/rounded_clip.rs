//! Rounded clips (flat-scene F3.2b, `docs/plans/flat-scene-f3.md`): a
//! box with `overflow: hidden` and a `border-radius` clips what is in it
//! to its padding box with the padding box's radii (its radii less its
//! border), and a hit outside them misses. The refusals are in
//! `rounded_clip_refused.rs`, the lanes in `rounded_clip_lanes.rs`.

use crate::support;

use support::rounded_page::{at, card, fill_clip};
use vapors_dom::Dom;
use vapors_scene::{Clip, Rect};

#[test]
fn what_a_rounded_hidden_box_holds_is_clipped_to_its_padding_radii() {
    for scale in [1.0, 2.0] {
        // Given
        let (mut doc, _) = card("", "");

        // When
        doc.update(at(scale)).expect("updates");

        // Then: the padding box, the radii less the 1px border
        let padding = Rect::new(scale, scale, 101.0 * scale, 61.0 * scale);
        let want = Clip::rounded(padding, [11.0 * scale; 4]);
        assert_eq!(fill_clip(&doc), Some(want), "at {scale}x");
    }
}

#[test]
fn a_clipping_box_clear_of_the_corners_gives_its_content_a_square_clip() {
    // Given: a clipping box 20px down the card, 20px tall
    let css = ".inner { overflow: hidden; margin-top: 20px; height: 20px; }";
    let (mut doc, _) = card(css, "inner");

    // When
    doc.update(at(1.0)).expect("updates");

    // Then
    let want = Clip::square(Rect::new(1.0, 21.0, 101.0, 41.0));
    assert_eq!(fill_clip(&doc), Some(want));
}

#[test]
fn a_radius_change_alone_paints_what_the_box_clips_again() {
    // Given
    let (mut doc, card) = card(".rounder { border-radius: 21px; }", "");
    doc.update(at(1.0)).expect("updates");

    // When
    doc.set_attribute(card, "class", "card rounder");
    doc.update(at(1.0)).expect("updates");

    // Then
    let want = Clip::rounded(Rect::new(1.0, 1.0, 101.0, 61.0), [20.0; 4]);
    assert_eq!(fill_clip(&doc), Some(want));
}

#[test]
fn a_radius_change_alone_paints_what_the_box_clips_deeper_down_again() {
    // Given: the fill inside a plain div inside the card
    let (mut doc, card) = card("", "plain");
    doc.update(at(1.0)).expect("updates");

    // When: the card's own style alone changes
    doc.set_attribute(card, "style", "border-radius: 21px");
    doc.update(at(1.0)).expect("updates");

    // Then
    let want = Clip::rounded(Rect::new(1.0, 1.0, 101.0, 61.0), [20.0; 4]);
    assert_eq!(fill_clip(&doc), Some(want));
}

#[test]
fn a_point_outside_a_rounded_corner_misses_what_the_box_clips() {
    // Given
    let (mut doc, card) = card("", "");
    doc.update(at(1.0)).expect("updates");
    let painted = doc.painted().expect("painted");
    let fill = painted.hit([50.0, 30.0]).expect("the fill is hit");

    // Then: inside the padding box's rectangle, outside its 11px curve,
    // the card is hit, not the fill; just inside the curve, the fill
    assert_eq!(painted.hit([2.0, 2.0]), Some(card));
    assert_eq!(painted.hit([5.0, 5.0]), Some(fill));
    assert_ne!(fill, card);
}
