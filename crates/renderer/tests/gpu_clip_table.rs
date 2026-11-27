//! GPU tier: the clip table (`docs/plans/flat-scene-f3.md`): a kept
//! quad drawn under a clip entry covers only the clip's pixels, with no
//! scissor; the clip moves with its frame's entry; a chain of two
//! intersects them; a row-relative clip moves with the row; a given-back
//! entry covers nothing (the relaxed backstop, item 11); a rounded entry
//! cuts its corners (F3.2, item 13).

#[path = "support/frame.rs"]
mod frame;
#[path = "support/table.rs"]
mod table;

use frame::{BACKGROUND, pixel, quad, square, target};
use table::{Table, table};
use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{ClipEntry, FrameEncoder, QuadPipeline, Target};

const SIDE: u32 = 16;
const RED: [u8; 4] = [255, 0, 0, 255];

/// The square after a red quad over all of it is kept and drawn in
/// `table`'s frame (row `row` of its rows), under the clip `clip(table)`
/// sets up and returns.
fn drawn_under(
    (shift, rows, row): ([f32; 2], &[[f32; 2]], u32),
    clip: impl FnOnce(&mut Table, &Target<'_>) -> u32,
) -> Vec<u8> {
    let gpu = gpu();
    let (texture, view) = square(&gpu, SIDE);
    let mut pipeline = QuadPipeline::new(&gpu.device, texture.format());
    let encoder = FrameEncoder::new(&gpu.device);
    let at = target(&gpu, &encoder, &view, SIDE);
    let mut table = table(&at, shift, rows);
    let index = clip(&mut table, &at);
    let rows = if rows.is_empty() { vec![] } else { vec![row] };
    let whole = [quad([-32.0, -32.0, 48.0, 48.0], RED)];
    let bytes = QuadPipeline::bytes_in(&whole, (table.frame, &[(0, index)]), &rows);
    let range = pipeline.keep(&at, &bytes).expect("room");
    let rows_base = table.rows_base;
    pipeline.draw_kept(
        &at,
        &range,
        (&mut table.transforms, &mut table.clips, rows_base),
    );
    encoder.submit(&gpu.queue).expect("no refusal");
    pipeline.release(range);
    read_rgba8(&gpu.device, &gpu.queue, &texture)
}

/// A clip entry `entry` taken and set in `table`.
fn clip_entry(table: &mut Table, at: &Target<'_>, entry: ClipEntry) -> u32 {
    let index = table.clips.take(at.encoder, 1).expect("room");
    table.clips.set(at.encoder, at.queue, index, entry);
    index
}

/// Which pixels of row `y` are red, as x.
fn red_xs(pixels: &[u8], y: u32) -> Vec<u32> {
    (0..SIDE)
        .filter(|&x| pixel(pixels, SIDE, x, y) == RED)
        .collect()
}

#[test]
fn gpu_a_quad_under_a_clip_entry_covers_only_the_clip() {
    // When: clipped to 4..12 by 6..10 in the page's frame
    let drawn = drawn_under(([0.0, 0.0], &[], 0), |table, at| {
        let frame = table.frame;
        clip_entry(table, at, ClipEntry::rect([4.0, 6.0, 12.0, 10.0], frame))
    });

    // Then
    assert_eq!(red_xs(&drawn, 7), (4..12).collect::<Vec<_>>());
    assert_eq!(pixel(&drawn, SIDE, 7, 5), BACKGROUND);
    assert_eq!(pixel(&drawn, SIDE, 7, 10), BACKGROUND);
}

#[test]
fn gpu_clip_zero_covers_everything() {
    // When
    let drawn = drawn_under(([0.0, 0.0], &[], 0), |_, _| 0);

    // Then
    assert_eq!(red_xs(&drawn, 0).len(), SIDE as usize);
}

#[test]
fn gpu_a_clip_moves_with_its_frame_s_entry() {
    // When: the frame shifted by 3, 2; the clip at 4..8 in the frame
    let drawn = drawn_under(([3.0, 2.0], &[], 0), |table, at| {
        let frame = table.frame;
        clip_entry(table, at, ClipEntry::rect([4.0, 4.0, 8.0, 8.0], frame))
    });

    // Then: at 7..11 on the target, rows 6..10
    assert_eq!(red_xs(&drawn, 7), (7..11).collect::<Vec<_>>());
    assert_eq!(pixel(&drawn, SIDE, 8, 5), BACKGROUND);
}

#[test]
fn gpu_a_chain_of_two_clips_covers_their_intersection() {
    // When: 2..10 inside 6..14, horizontally
    let drawn = drawn_under(([0.0, 0.0], &[], 0), |table, at| {
        let frame = table.frame;
        let outer = clip_entry(table, at, ClipEntry::rect([6.0, 0.0, 14.0, 16.0], frame));
        let inner = ClipEntry {
            parent: outer,
            ..ClipEntry::rect([2.0, 0.0, 10.0, 16.0], frame)
        };
        clip_entry(table, at, inner)
    });

    // Then
    assert_eq!(red_xs(&drawn, 3), (6..10).collect::<Vec<_>>());
}

#[test]
fn gpu_a_row_relative_clip_moves_with_the_instance_s_row() {
    // When: row 1 is 5 px down; the clip, 0..4 high relative to the row
    let drawn = drawn_under(([0.0, 0.0], &[[0.0, 0.0], [0.0, 5.0]], 1), |table, at| {
        let frame = table.frame;
        let entry = ClipEntry {
            row_relative: true,
            ..ClipEntry::rect([0.0, 0.0, 16.0, 4.0], frame)
        };
        clip_entry(table, at, entry)
    });

    // Then: rows 5..9 on the target
    let red_rows: Vec<u32> = (0..SIDE)
        .filter(|&y| pixel(&drawn, SIDE, 0, y) == RED)
        .collect();
    assert_eq!(red_rows, (5..9).collect::<Vec<_>>());
}

#[test]
fn gpu_a_given_back_clip_entry_covers_nothing() {
    // When: an entry set, given back (written with the poison), and named
    let drawn = drawn_under(([0.0, 0.0], &[], 0), |table, at| {
        let frame = table.frame;
        let index = clip_entry(table, at, ClipEntry::rect([0.0, 0.0, 16.0, 16.0], frame));
        table.clips.give(index, 1);
        index
    });

    // Then
    assert!(
        red_xs(&drawn, 8).is_empty(),
        "a poisoned clip covered pixels"
    );
}

#[test]
fn gpu_a_rounded_clip_entry_cuts_the_corners_off() {
    // When: clipped to the whole square with a radius of 8 at each corner,
    // a circle of the square's side
    let drawn = drawn_under(([0.0, 0.0], &[], 0), |table, at| {
        let frame = table.frame;
        let entry = ClipEntry::rounded([0.0, 0.0, 16.0, 16.0], [8.0; 4], frame);
        clip_entry(table, at, entry)
    });

    // Then: the corners empty, the middle full, as a square clip's are not
    for (x, y) in [(0, 0), (15, 0), (15, 15), (0, 15), (1, 1)] {
        assert_eq!(
            pixel(&drawn, SIDE, x, y),
            BACKGROUND,
            "corner pixel {x},{y}"
        );
    }
    assert_eq!(pixel(&drawn, SIDE, 8, 8), RED);
    let across = red_xs(&drawn, 8);
    assert!((2..14).all(|x| across.contains(&x)), "{across:?}");
}
