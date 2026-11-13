use super::{FrameInputs, Frames};
use crate::frames::PAGE;
use crate::kept_clips::Clips;
use crate::layout::BoxRect;

fn rect(x: f32, y: f32) -> BoxRect {
    BoxRect {
        x,
        y,
        width: 50.0,
        height: 50.0,
    }
}

/// Slot 1 scrolls on the page with slot 2 scrolling inside it; slots 3
/// to 102 each scroll on the page. Each clips its content to a 50 px
/// square.
fn page_clips() -> Clips {
    let mut frame = vec![PAGE, PAGE, 1];
    frame.extend((3..=102).map(|_| PAGE));
    let inner = (0..frame.len())
        .map(|slot| {
            (slot > 0).then(|| {
                rect(
                    0.0,
                    60.0 * f32::from(u16::try_from(slot).expect("few slots")),
                )
            })
        })
        .collect();
    Clips::with(inner, frame)
}

fn depth(slot: usize) -> usize {
    usize::from(slot == 2) + 1
}

/// The frames of `clips`' scroll containers, slot 1 shifted `outer`.
fn made(clips: &Clips, outer: [f32; 2]) -> Frames {
    let shift = move |slot: usize| if slot == 1 { outer } else { [0.0; 2] };
    let inputs = FrameInputs {
        clips,
        depth: &depth,
        shift: &shift,
    };
    Frames::of(&(1..=102).collect::<Vec<_>>(), &inputs, 1.0)
}

/// `frames` updated for `changed`, slot 1 shifted `outer`; whether a
/// frame moved, and how many were worked out.
fn updated(
    frames: &mut Frames,
    clips: &Clips,
    changed: &[usize],
    outer: [f32; 2],
) -> (bool, usize) {
    let shift = move |slot: usize| if slot == 1 { outer } else { [0.0; 2] };
    let inputs = FrameInputs {
        clips,
        depth: &depth,
        shift: &shift,
    };
    frames.update(changed, &inputs)
}

#[test]
fn a_scrolled_frame_and_the_frames_nested_in_it_are_worked_out_again_and_no_other() {
    // Given
    let clips = page_clips();
    let mut frames = made(&clips, [0.0; 2]);

    // When: slot 1 scrolled 30 px
    let (moved, worked) = updated(&mut frames, &clips, &[1], [0.0, -30.0]);

    // Then
    assert!(moved);
    assert_eq!(worked, 2);
    assert_eq!(frames, made(&clips, [0.0, -30.0]));
}

#[test]
fn a_frame_that_comes_out_the_same_works_out_no_frame_nested_in_it() {
    // Given
    let clips = page_clips();
    let mut frames = made(&clips, [0.0; 2]);

    // When: slot 1 named, but its shift and clip unchanged
    let (moved, worked) = updated(&mut frames, &clips, &[1], [0.0; 2]);

    // Then
    assert!(!moved);
    assert_eq!(worked, 1);
}

#[test]
fn a_frame_named_twice_or_also_reached_from_its_outer_frame_is_worked_out_once() {
    // Given
    let clips = page_clips();
    let mut frames = made(&clips, [0.0; 2]);

    // When: the inner frame named before and after its outer one
    let (moved, worked) = updated(&mut frames, &clips, &[2, 1, 2], [0.0, -30.0]);

    // Then
    assert!(moved);
    assert_eq!(worked, 2);
    assert_eq!(frames, made(&clips, [0.0, -30.0]));
}
