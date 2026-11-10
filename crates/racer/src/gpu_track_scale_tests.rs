//! GPU tier: the track shows the same colours at device scale 1 and 2
//! for the same CSS size, sampled on flat sky or road, an edge and a
//! gradient (docs/gpu-surface.md, 3D slice 1c PR 6).
use crate::gpu_frames::{WINDOW, pixel_in, track_alone_in};
use crate::gpu_track_resize_tests::{DOUBLE, drawn_in, racing};
use vapors_gpu_harness::gpu;

/// Largest per-channel difference between a pixel and its right and
/// lower neighbours: how steep the colour is there.
fn steepness(rgba: &[u8], [x, y]: [f32; 2]) -> u8 {
    let here = pixel_in(rgba, WINDOW.width, [x, y]);
    let next = [[x + 1.0, y], [x, y + 1.0]].map(|at| pixel_in(rgba, WINDOW.width, at));
    let diff = |a: [u8; 4], b: [u8; 4]| (0..3).map(|c| a[c].abs_diff(b[c])).max().unwrap_or(0);
    next.iter().map(|n| diff(here, *n)).max().unwrap_or(0)
}

/// CSS points where the 1x frame shows the track alone, with the
/// track's steepness there: the HUD covers none of them, nor the pixel
/// after them in either direction.
fn track_points(single: &[u8], alone: &[u8]) -> Vec<([f32; 2], u8)> {
    let clear = |at: [f32; 2]| {
        let [x, y] = at;
        [[x, y], [x + 1.0, y], [x, y + 1.0]]
            .iter()
            .all(|p| pixel_in(single, WINDOW.width, *p) == pixel_in(alone, WINDOW.width, *p))
    };
    let (width, height) = (WINDOW.width as usize, WINDOW.height as usize);
    let grid = (8..height - 8)
        .step_by(8)
        .flat_map(|y| (8..width - 8).step_by(8).map(move |x| [x as f32, y as f32]));
    grid.filter(|at| clear(*at))
        .map(|at| (at, steepness(alone, at)))
        .collect()
}

/// The flat point, the steepest point, and the steepest point under
/// `GRADUAL`: sky or road, an edge, and a gradient.
fn sampled(points: &[([f32; 2], u8)]) -> [[f32; 2]; 3] {
    /// Steeper than this per device pixel is an edge, not a gradient.
    const GRADUAL: u8 = 16;
    let pick = |keep: &dyn Fn(u8) -> bool| {
        let best = points
            .iter()
            .filter(|(_, s)| keep(*s))
            .max_by_key(|(_, s)| *s);
        best.expect("the track has points of this kind clear of the HUD")
            .0
    };
    let flat = points.iter().find(|(_, s)| *s == 0).expect("flat track").0;
    [flat, pick(&|_| true), pick(&|s| s <= GRADUAL)]
}

/// Per channel, the range of the 3x3 device pixels at 1x around `at`.
fn range_around(rgba: &[u8], [x, y]: [f32; 2]) -> [(u8, u8); 3] {
    let around: Vec<[u8; 4]> = (-1..=1)
        .flat_map(|dy| (-1..=1).map(move |dx| [x + dx as f32, y + dy as f32]))
        .map(|at| pixel_in(rgba, WINDOW.width, at))
        .collect();
    [0, 1, 2].map(|c| {
        let values = around.iter().map(|p| p[c]);
        (values.clone().min().unwrap_or(0), values.max().unwrap_or(0))
    })
}

/// Per channel, the mean of the 2x2 device pixels at 2x that cover `at`.
fn mean_block(rgba: &[u8], [x, y]: [f32; 2]) -> [f32; 3] {
    let block = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
    let pixels = block.map(|[dx, dy]| pixel_in(rgba, DOUBLE.width, [x * 2.0 + dx, y * 2.0 + dy]));
    [0, 1, 2].map(|c| pixels.iter().map(|p| f32::from(p[c])).sum::<f32>() / 4.0)
}

#[test]
fn gpu_the_track_shows_the_same_colour_at_one_and_two_times_scale() {
    // Given: racing, the clock held at `now`, so the scene does not move
    let gpu = gpu();
    let (mut racer, now) = racing(&gpu);
    let single = drawn_in(&mut racer, &gpu, WINDOW, now);
    let alone = track_alone_in(&racer, &gpu, WINDOW);
    let points = sampled(&track_points(&single, &alone));

    // When
    let double = drawn_in(&mut racer, &gpu, DOUBLE, now);

    // Then: at each CSS point, the mean of the 2x block lies within the
    // range of the 1x pixels around it, widened by TOLERANCE. The 1x
    // pixel and the block are sampled up to one device pixel apart, so
    // on an edge or gradient they can differ by what the colour changes
    // across that pixel; the range is that change. TOLERANCE is 8-bit
    // rounding of the mean.
    const TOLERANCE: f32 = 2.0;
    for at in points {
        let colour = pixel_in(&single, WINDOW.width, at);
        assert_ne!(
            colour,
            [255, 0, 255, 255],
            "the track drew nothing at {at:?}"
        );
        let mean = mean_block(&double, at);
        for (channel, (low, high)) in range_around(&single, at).into_iter().enumerate() {
            let inside = mean[channel] >= f32::from(low) - TOLERANCE
                && mean[channel] <= f32::from(high) + TOLERANCE;
            assert!(
                inside,
                "{at:?} channel {channel}: 2x mean {} outside {low}..={high}",
                mean[channel]
            );
        }
    }
}
