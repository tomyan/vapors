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

#[test]
fn a_view_visits_the_page_and_only_the_frames_whose_clip_meets_it() {
    // Given: slots 10 and 11's clips are at 600 and 660, 50 px tall
    let frames = made(&page_clips(), [0.0; 2]);

    // When
    let mut found = frames.meeting(vapors_scene::Rect::new(0.0, 620.0, 100.0, 680.0));

    // Then
    found.sort_unstable();
    assert_eq!(found, [10, 11, PAGE]);
}

#[test]
fn a_frame_nested_in_a_scrolled_frame_is_found_where_the_scroll_put_it() {
    // Given: slot 2's clip is at 120 in slot 1's frame, below slot 1's
    // clip (60 to 110 on the page) until slot 1 scrolls 80 px
    let frames = made(&page_clips(), [0.0, -80.0]);

    // When
    let mut found = frames.meeting(vapors_scene::Rect::new(0.0, 70.0, 100.0, 80.0));

    // Then
    found.sort_unstable();
    assert_eq!(found, [1, 2, PAGE]);
}

#[test]
#[should_panic(
    expected = "scroll container slot 500 is updated but the kept frames were not made for it"
)]
fn updating_a_container_the_frames_were_not_made_for_is_refused_naming_it() {
    // Given
    let clips = page_clips();
    let mut frames = made(&clips, [0.0; 2]);

    // When
    updated(&mut frames, &clips, &[500], [0.0; 2]);
}

/// Frames for slot 1 scrolling on the page, unclipped in effect,
/// shifted `shift` device pixels, and slot 2 in it clipped to 50 x 11
/// CSS pixels at `top`, at `scale`.
fn nested_at(scale: f32, shift: f32, top: f32) -> Frames {
    let clips = Clips::with(
        vec![
            None,
            Some(BoxRect {
                x: 0.0,
                y: -1e5,
                width: 1e5,
                height: 2e5,
            }),
            Some(BoxRect {
                x: 0.0,
                y: top,
                width: 50.0,
                height: 11.0,
            }),
        ],
        vec![PAGE, PAGE, 1],
    );
    let own = move |slot: usize| if slot == 1 { [0.0, shift] } else { [0.0; 2] };
    let inputs = FrameInputs {
        clips: &clips,
        depth: &depth,
        shift: &own,
    };
    Frames::of(&[1, 2], &inputs, scale)
}

#[test]
fn a_nested_frames_clip_lies_inside_its_indexed_clip_at_fractional_scales() {
    for scale in [1.25, 1.5, 2.625] {
        for shift in (0..400u16).map(|s| -f32::from(s)) {
            for top in (0..20u16).map(|k| f32::from(2 * k + 1)) {
                // Given
                let frames = nested_at(scale, shift, top);

                // When
                let clip = frames.at(2).device_clip.expect("clipped");
                let entered = frames.entered[&2]
                    .expect("entered")
                    .offset(frames.at(1).shift);

                // Then
                let inside = clip.top >= entered.top && clip.bottom <= entered.bottom;
                let inside = inside && clip.left >= entered.left && clip.right <= entered.right;
                assert!(
                    inside,
                    "scale {scale}, shift {shift}, top {top}: {clip:?} outside {entered:?}"
                );
            }
        }
    }
}
