//! The moves `kept_shift.rs` refuses: each is painted again.

use vapors_dom::NodeId;
use vapors_scene::{Color, Item, Rect, Shadow};

use super::tests::{from, kept, painting, rect, shift_of};

#[test]
fn an_item_under_another_clip_than_the_parts_is_painted_again() {
    // Given
    let mut part = kept(painting(rect(0.0, 0.0)), rect(0.0, 0.0));
    part.own
        .scene
        .clips_mut()
        .fill(Some(Rect::new(0.0, 0.0, 5.0, 5.0)));

    // Then: the part's clip is none, its item's is not
    assert!(shift_of(&part, from(rect(0.0, 24.0))).is_none());
}

#[test]
fn a_fractional_position_is_painted_again() {
    // Given
    let part = kept(painting(rect(0.0, 0.0)), rect(0.0, 0.0));

    // When
    let shift = shift_of(&part, from(rect(0.0, 24.5)));

    // Then
    assert!(shift.is_none());
}

#[test]
fn a_size_change_is_painted_again() {
    // Given
    let part = kept(painting(rect(0.0, 0.0)), rect(0.0, 0.0));
    let mut to = from(rect(0.0, 24.0));
    to.rect.width = 12.0;

    // Then
    assert!(shift_of(&part, to).is_none());
}

#[test]
fn a_padding_change_is_painted_again() {
    // Given
    let part = kept(painting(rect(0.0, 0.0)), rect(0.0, 0.0));
    let mut to = from(rect(0.0, 24.0));
    to.padding.left = 2.0;

    // Then
    assert!(shift_of(&part, to).is_none());
}

#[test]
fn an_item_off_whole_pixels_is_painted_again() {
    // Given: a shadow offset by half a pixel
    let mut painting = painting(rect(0.0, 0.0));
    painting.scene.push(Item::Shadow(Shadow {
        rect: Rect::new(0.5, 0.5, 10.5, 10.5),
        radii: [0.0; 4],
        blur: 2.0,
        color: Color::default(),
        caster: Rect::new(0.0, 0.0, 10.0, 10.0),
        caster_radii: [0.0; 4],
    }));
    let part = kept(painting, rect(0.0, 0.0));

    // Then
    assert!(shift_of(&part, from(rect(0.0, 24.0))).is_none());
}

#[test]
fn a_part_with_an_inline_elements_rect_is_painted_again() {
    // Given
    let mut painting = painting(rect(0.0, 0.0));
    painting.rects.insert(NodeId(8), rect(2.0, 2.0));
    let part = kept(painting, rect(0.0, 0.0));

    // Then
    assert!(shift_of(&part, from(rect(0.0, 24.0))).is_none());
}

#[test]
fn an_elements_part_with_two_hits_is_painted_again() {
    // Given
    let mut painting = painting(rect(0.0, 0.0));
    painting.hits.push((NodeId(8), rect(2.0, 2.0)));
    let part = kept(painting, rect(0.0, 0.0));

    // Then
    assert!(shift_of(&part, from(rect(0.0, 24.0))).is_none());
}

#[test]
fn a_text_item_in_an_elements_part_is_painted_again() {
    // Given
    let mut painting = painting(rect(0.0, 0.0));
    painting.scene.push(Item::Text {
        text: vapors_scene::TextId(0),
        origin: [0.0, 0.0],
        ink: Rect::new(0.0, 0.0, 1.0, 1.0),
    });
    let part = kept(painting, rect(0.0, 0.0));

    // Then
    assert!(shift_of(&part, from(rect(0.0, 24.0))).is_none());
}
