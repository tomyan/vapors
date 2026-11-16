use vapors_scene::Rect;

use super::FrameDraw;

#[test]
fn a_batchs_clip_in_its_frame_is_moved_by_the_shift_and_cut_to_the_frames_clip() {
    // Given: a frame shifted up 30 under a clip from 0 to 50
    let frame = FrameDraw {
        shift: [0.0, -30.0],
        clip: Some(Rect::new(0.0, 0.0, 100.0, 50.0)),
    };

    // When
    let clipped = frame.on_page(Some(Rect::new(10.0, 40.0, 90.0, 100.0)));
    let unclipped = frame.on_page(None);

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
    assert_eq!(page.on_page(clip), clip);
}
