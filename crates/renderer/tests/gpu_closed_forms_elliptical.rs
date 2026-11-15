//! GPU tier: a rect with elliptical corners (rx 8, ry 4 device pixels)
//! is covered by its distance in device pixels, against a reference
//! from the exact distance to each quarter ellipse. The shader takes
//! the corner's distance to first order (`ellipse_distance`); the
//! bounds are that error's maximum within the ramp, measured from the
//! exact distance at 1/4 px steps across it, plus the 1.5 / 255 the
//! other closed-form tests allow for rounding: 0.013 px filled, 0.057
//! px for a 1 px stroke, whose inner ramp reaches 1 px inside, where
//! the first-order error grows.

#[path = "support/closed_forms.rs"]
mod support;

use support::{SIDE, coverage, scaled, white};
use vapors_shapes::Geometry;

const ORIGIN: [f32; 2] = [4.25, 6.5];
const SIZE: [f32; 2] = [22.0, 18.0];
const RADII: [f32; 2] = [8.0, 4.0];

/// The rect at `scale` device pixels per unit.
fn elliptical(scale: f32) -> Geometry<'static> {
    Geometry::Rect {
        origin: ORIGIN.map(|v| v / scale),
        size: SIZE.map(|v| v / scale),
        radius: RADII.map(|v| v / scale),
    }
}

/// The exact signed distance from `p` to the rect, device pixels.
fn exact_distance(p: [f32; 2]) -> f32 {
    let half = SIZE.map(|v| v / 2.0);
    let folded = [0, 1].map(|i| (p[i] - ORIGIN[i] - half[i]).abs());
    let corner = [0, 1].map(|i| half[i] - RADII[i]);
    if folded[0] > corner[0] && folded[1] > corner[1] {
        return quarter_ellipse_distance([folded[0] - corner[0], folded[1] - corner[1]]);
    }
    (folded[0] - half[0]).max(folded[1] - half[1])
}

/// The distance from `q` (from the corner's centre) to the quarter
/// ellipse of `RADII`, from 20 000 points along it: within 0.001 px.
fn quarter_ellipse_distance(q: [f32; 2]) -> f32 {
    let nearest = (0..=20_000)
        .map(|i| f64::from(i) / 20_000.0 * std::f64::consts::FRAC_PI_2)
        .map(|t| {
            let on = [f64::from(RADII[0]) * t.cos(), f64::from(RADII[1]) * t.sin()];
            (f64::from(q[0]) - on[0]).hypot(f64::from(q[1]) - on[1])
        })
        .fold(f64::INFINITY, f64::min);
    let inside = (q[0] / RADII[0]).hypot(q[1] / RADII[1]) < 1.0;
    #[allow(clippy::cast_possible_truncation)]
    let nearest = nearest as f32;
    if inside { -nearest } else { nearest }
}

/// The largest difference from the reference coverage, `outside`
/// mapping the exact distance to the distance past the drawn edge, and
/// the pixel it is at.
fn worst(coverage: &[f32], outside: impl Fn(f32) -> f32) -> (f32, (u32, u32)) {
    let mut worst = (0.0, (0, 0));
    for y in 0..SIDE {
        for x in 0..SIDE {
            #[allow(clippy::cast_precision_loss)]
            let centre = [x as f32 + 0.5, y as f32 + 0.5];
            let expected = (0.5 - outside(exact_distance(centre))).clamp(0.0, 1.0);
            let off = (coverage[(y * SIDE + x) as usize] - expected).abs();
            if off > worst.0 {
                worst = (off, (x, y));
            }
        }
    }
    worst
}

#[test]
fn gpu_a_filled_rect_with_elliptical_corners_is_covered_by_its_distance() {
    for scale in [1.0, 2.0] {
        // When
        let coverage = coverage(&[white(&elliptical(scale), None, scaled(scale))]);

        // Then
        let (off, at) = worst(&coverage, |d| d);
        assert!(off <= 0.013 + 1.5 / 255.0, "{scale}x: {off} off at {at:?}");
    }
}

#[test]
fn gpu_a_stroked_rect_with_elliptical_corners_is_covered_by_its_distance() {
    for scale in [1.0, 2.0] {
        // When: stroked 1 device pixel wide
        let shape = white(&elliptical(scale), Some(1.0 / scale), scaled(scale));
        let coverage = coverage(&[shape]);

        // Then
        let (off, at) = worst(&coverage, |d| d.abs() - 0.5);
        assert!(off <= 0.057 + 1.5 / 255.0, "{scale}x: {off} off at {at:?}");
    }
}
