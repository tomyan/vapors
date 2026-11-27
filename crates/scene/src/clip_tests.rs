use super::Clip;
use crate::display::Rect;

const FRAME: Rect = Rect {
    left: 0.0,
    top: 0.0,
    right: 100.0,
    bottom: 50.0,
};

fn rounded(top: f32) -> Clip {
    Clip::rounded(
        Rect::new(10.0, top, 90.0, top + 40.0),
        [12.0, 12.0, 0.0, 0.0],
    )
}

#[test]
fn a_square_clip_cut_by_a_frame_is_their_intersection() {
    // When
    let cut = Clip::square(Rect::new(10.0, 30.0, 90.0, 70.0)).cut(FRAME);

    // Then
    assert_eq!(cut, Clip::square(Rect::new(10.0, 30.0, 90.0, 50.0)));
}

#[test]
fn a_rounded_clip_inside_a_frame_is_unchanged_by_it() {
    assert_eq!(rounded(5.0).cut(FRAME), rounded(5.0));
}

#[test]
fn a_rounded_clip_past_a_frame_keeps_its_corners_and_takes_the_frame_as_outer() {
    // When: cut at the bottom, below its rounded top corners
    let cut = rounded(30.0).cut(FRAME);

    // Then: its own rectangle and corners, and the frame
    assert_eq!(
        (cut.rect, cut.radii),
        (rounded(30.0).rect, rounded(30.0).radii)
    );
    assert_eq!(cut.outer, Some(FRAME));
    assert_eq!(cut.bounds(), Rect::new(10.0, 30.0, 90.0, 50.0));
}

#[test]
fn a_second_cut_narrows_the_outer_rectangle() {
    // When
    let twice = rounded(30.0)
        .cut(FRAME)
        .cut(Rect::new(0.0, 0.0, 50.0, 100.0));

    // Then
    assert_eq!(twice.outer, Some(Rect::new(0.0, 0.0, 50.0, 50.0)));
}

#[test]
fn an_offset_moves_the_rectangle_and_the_outer_one() {
    // When
    let moved = rounded(30.0).cut(FRAME).offset([1.0, 2.0]);

    // Then
    assert_eq!(moved.rect, Rect::new(11.0, 32.0, 91.0, 72.0));
    assert_eq!(moved.outer, Some(Rect::new(1.0, 2.0, 101.0, 52.0)));
    assert_eq!(moved.radii, [12.0, 12.0, 0.0, 0.0]);
}
