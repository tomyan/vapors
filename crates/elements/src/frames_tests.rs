use vapors_scene::{Color, Item, Quad, Rect, TextId};

use super::{FrameInputs, Frames, PAGE};
use crate::kept_clips::Clips;
use crate::layout::BoxRect;

fn rect(x: f32, y: f32, width: f32, height: f32) -> BoxRect {
    BoxRect {
        x,
        y,
        width,
        height,
    }
}

/// Slot 1 scrolls on the page, clipping its content to 100 x 100 at
/// (10, 10); slot 2 scrolls inside slot 1, clipping its content to its
/// padding box at (20, 200) in slot 1's frame. Slot 1 is shifted
/// `outer`, slot 2 `inner`, device pixels.
fn nested(outer: [f32; 2], inner: [f32; 2], scale: f32) -> Frames {
    let clips = Clips::with(
        vec![
            None,
            Some(rect(10.0, 10.0, 100.0, 100.0)),
            Some(rect(20.0, 200.0, 50.0, 50.0)),
        ],
        vec![PAGE, PAGE, 1],
    );
    let shift = |slot: usize| if slot == 1 { outer } else { inner };
    let inputs = FrameInputs {
        clips: &clips,
        depth: &|slot| slot,
        shift: &shift,
    };
    Frames::of(&[2, 1], &inputs, scale)
}

#[test]
fn a_rect_in_a_scrolled_frame_is_on_the_page_moved_by_its_shift() {
    // Given: slot 1's content scrolled down 30 px
    let frames = nested([0.0, -30.0], [0.0; 2], 1.0);

    // When
    let on_page = frames.rect(1, rect(10.0, 50.0, 20.0, 20.0));

    // Then
    assert_eq!(on_page, rect(10.0, 20.0, 20.0, 20.0));
    assert_eq!(frames.point(1, [5.0, 5.0]), [5.0, -25.0]);
    assert_eq!(
        frames.rect(PAGE, rect(1.0, 2.0, 3.0, 4.0)),
        rect(1.0, 2.0, 3.0, 4.0)
    );
}

#[test]
fn a_shift_in_device_pixels_moves_css_positions_by_it_over_the_scale() {
    // Given: a shift of 30 device pixels at scale 2
    let frames = nested([0.0, -30.0], [0.0; 2], 2.0);

    // When
    let on_page = frames.point(1, [0.0, 50.0]);

    // Then
    assert_eq!(on_page, [0.0, 35.0]);
}

#[test]
fn a_nested_frame_adds_its_shift_to_the_outer_one_and_is_cut_to_its_clip() {
    // Given: slot 1 scrolled 30 px down, slot 2 inside it 5 px down
    let frames = nested([0.0, -30.0], [0.0, -5.0], 1.0);

    // When
    let inner = *frames.at(2);

    // Then: slot 2's clip, (20, 200) in slot 1's frame, is at
    // (20, 170) on the page, below slot 1's clip, which ends at 110
    assert_eq!(inner.shift, [0.0, -35.0]);
    assert_eq!(inner.clip, Some(rect(20.0, 170.0, 50.0, 0.0)));
    assert_eq!(frames.at(1).clip, Some(rect(10.0, 10.0, 100.0, 100.0)));
}

#[test]
fn an_item_is_moved_by_its_frame_and_cut_to_its_frame_clip() {
    // Given
    let frames = nested([0.0, -30.0], [0.0; 2], 1.0);
    let quad = Item::Quad(Quad::fill(
        Rect::new(0.0, 40.0, 50.0, 60.0),
        Color::from_rgba8(0, 0, 0, 255),
    ));

    // When
    let (moved, clip) = frames.item(1, &quad, Some(Rect::new(0.0, 0.0, 200.0, 100.0)));
    let (_, unclipped) = frames.item(1, &quad, None);

    // Then
    let Item::Quad(moved) = moved else {
        panic!("a quad stays a quad")
    };
    assert_eq!(moved.rect, Rect::new(0.0, 10.0, 50.0, 30.0));
    assert_eq!(clip, Some(Rect::new(10.0, 10.0, 110.0, 70.0)));
    assert_eq!(unclipped, Some(Rect::new(10.0, 10.0, 110.0, 110.0)));
}

#[test]
fn a_text_item_moves_its_origin_and_ink() {
    // Given
    let frames = nested([-4.0, -30.0], [0.0; 2], 1.0);
    let text = Item::Text {
        text: TextId(0),
        origin: [10.5, 40.25],
        ink: Rect::new(10.0, 40.0, 30.0, 52.0),
    };

    // When
    let (moved, _) = frames.item(1, &text, None);

    // Then
    let Item::Text { origin, ink, .. } = moved else {
        panic!("a text stays a text")
    };
    assert_eq!(origin, [6.5, 10.25]);
    assert_eq!(ink, Rect::new(6.0, 10.0, 26.0, 22.0));
}

#[test]
fn a_hit_is_moved_by_its_frame_and_cut_to_its_frame_clip() {
    // Given
    let frames = nested([0.0, -30.0], [0.0; 2], 1.0);

    // When
    let hit = frames.hit(1, rect(0.0, 100.0, 50.0, 50.0));

    // Then: (0, 70) to (50, 120) cut to (10, 10) to (110, 110)
    assert_eq!(hit, rect(10.0, 70.0, 40.0, 40.0));
}

#[test]
#[should_panic(expected = "names scroll frame 7, which this paint does not hold")]
fn a_frame_the_paint_does_not_hold_is_refused_by_name() {
    // Given
    let frames = nested([0.0; 2], [0.0; 2], 1.0);

    // When
    let _ = frames.rect(7, rect(0.0, 0.0, 1.0, 1.0));
}

#[test]
fn an_item_in_an_unshifted_frame_keeps_a_negative_zero_edge() {
    // Given: a quad whose top rounded to -0.0, in a frame not shifted
    let frames = nested([0.0; 2], [0.0; 2], 1.0);
    let quad = Item::Quad(Quad::fill(
        Rect::new(0.0, -0.0, 8.0, 8.0),
        Color::from_rgba8(0, 0, 0, 255),
    ));

    // When
    let (kept, _) = frames.item(1, &quad, None);

    // Then: the same bits, which the kept batches' keys compare
    let Item::Quad(kept) = kept else {
        panic!("a quad stays a quad")
    };
    assert!(kept.rect.top.is_sign_negative());
}
