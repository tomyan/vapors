use super::*;

const VIEWPORT: Size = Size {
    width: 100,
    height: 100,
};

fn scissor(x: u32, y: u32, width: u32, height: u32) -> Scissor {
    Scissor {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn an_unclipped_batch_draws_where_the_target_does() {
    assert_eq!(region(None, None, VIEWPORT), Region::Whole);
    let outer = scissor(5, 5, 10, 10);
    assert_eq!(region(Some(outer), None, VIEWPORT), Region::Within(outer));
}

#[test]
fn a_clipped_batch_draws_inside_its_clip_and_the_targets_scissor() {
    // Given
    let clip = Rect::new(10.0, 10.0, 50.0, 50.0);

    // When / Then
    assert_eq!(
        region(None, Some(clip), VIEWPORT),
        Region::Within(scissor(10, 10, 40, 40))
    );
    assert_eq!(
        region(Some(scissor(0, 0, 20, 100)), Some(clip), VIEWPORT),
        Region::Within(scissor(10, 10, 10, 40))
    );
}

#[test]
fn a_batch_clipped_to_nothing_is_not_drawn() {
    // Given: a clip off the target, and one outside the target's scissor
    let off = Rect::new(200.0, 0.0, 300.0, 10.0);
    let beside = Rect::new(50.0, 0.0, 60.0, 10.0);

    // When / Then
    assert_eq!(region(None, Some(off), VIEWPORT), Region::Nowhere);
    assert_eq!(
        region(Some(scissor(0, 0, 20, 20)), Some(beside), VIEWPORT),
        Region::Nowhere
    );
}

#[test]
fn a_batchs_clip_in_its_frame_is_moved_by_the_shift_and_cut_to_the_frames_clip() {
    // Given: a frame shifted up 30 under a clip from 0 to 50
    let frame = FrameDraw {
        shift: [0.0, -30.0],
        clip: Some(Rect::new(0.0, 0.0, 100.0, 50.0)),
    };

    // When
    let clipped = on_page(Some(Rect::new(10.0, 40.0, 90.0, 100.0)), frame);
    let unclipped = on_page(None, frame);

    // Then
    assert_eq!(clipped, Some(Rect::new(10.0, 10.0, 90.0, 50.0)));
    assert_eq!(unclipped, frame.clip);
}

#[test]
fn a_batch_in_the_pages_frame_keeps_its_clip() {
    // Given
    let page = FrameDraw {
        shift: [0.0; 2],
        clip: None,
    };
    let clip = Some(Rect::new(10.0, 40.0, 90.0, 100.0));

    // When / Then
    assert_eq!(on_page(clip, page), clip);
}
