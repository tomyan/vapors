//! GPU tier: rects with elliptical corners and ellipses, at radii
//! ratios to 8:1 either way and strokes deeper than the smaller radius,
//! under 1x, 2x, a fractional offset, a rotation and a skewed non-uniform
//! scale, are covered by their distance in device pixels to within 0.05
//! px of the exact distance (#386 review, H1, M1, M2). The reference
//! (`support/exact_forms.rs`) finds the nearest point by search, in
//! f64. It maps a distance to device pixels as the shader does:
//! divided by the shape units per device pixel along the exact
//! gradient. So a gradient pointing the wrong way fails under the skew,
//! and so does a jump where a corner meets its edges. 1.5 / 255 is
//! added to 0.05 for rounding, as the other closed-form tests allow.

#[path = "support/exact_forms.rs"]
mod exact_forms;
#[path = "support/closed_forms.rs"]
mod support;

use support::{SIDE, coverage, white};
use vapors_shapes::Geometry;

/// The largest coverage error accepted: 0.05 px of distance, and the
/// rounding.
const BOUND: f32 = 0.05 + 1.5 / 255.0;

struct Case {
    name: &'static str,
    geometry: Geometry<'static>,
    stroke: Option<f32>,
    transform: [f32; 6],
}

/// A rect of `size` with corners of `radii`, centred on the origin.
const fn rect(size: [f32; 2], radii: [f32; 2]) -> Geometry<'static> {
    Geometry::Rect {
        origin: [-size[0] / 2.0, -size[1] / 2.0],
        size,
        radius: radii,
    }
}

const fn ellipse(radii: [f32; 2]) -> Geometry<'static> {
    Geometry::Ellipse {
        centre: [0.0, 0.0],
        radii,
    }
}

/// `linear` ([a, b, c, d]) placing the origin at `at`.
const fn placed(linear: [f32; 4], at: [f32; 2]) -> [f32; 6] {
    [linear[0], linear[1], linear[2], linear[3], at[0], at[1]]
}

const ONE: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const TWO: [f32; 4] = [2.0, 0.0, 0.0, 2.0];
/// rotate(30deg): cos 30 = 0.8660254, sin 30 = 0.5.
const ROTATED: [f32; 4] = [0.866_025_4, 0.5, -0.5, 0.866_025_4];
/// A non-uniform scale with skew, under which the units per pixel
/// along a gradient depend on its direction, signs included.
const SKEWED: [f32; 4] = [1.6, 0.3, 0.5, 0.9];
const CENTRE: [f32; 2] = [16.0, 16.0];

const fn case(
    name: &'static str,
    geometry: Geometry<'static>,
    stroke: Option<f32>,
    transform: [f32; 6],
) -> Case {
    Case {
        name,
        geometry,
        stroke,
        transform,
    }
}

const CASES: [Case; 12] = [
    case(
        "8:1 corners filled",
        rect([24.0, 14.0], [8.0, 1.0]),
        None,
        placed(ONE, CENTRE),
    ),
    case(
        "8:1 corners, stroke 3",
        rect([24.0, 14.0], [8.0, 1.0]),
        Some(3.0),
        placed(ONE, CENTRE),
    ),
    case(
        "1:8 corners, stroke 3",
        rect([14.0, 24.0], [1.0, 8.0]),
        Some(3.0),
        placed(ONE, CENTRE),
    ),
    case(
        "3 x 1.5 corners, stroke 1.5",
        rect([20.0, 12.0], [3.0, 1.5]),
        Some(1.5),
        placed(ONE, [16.25, 16.4]),
    ),
    case(
        "3 x 1.5 corners, stroke 1.5, moved",
        rect([20.0, 12.0], [3.0, 1.5]),
        Some(1.5),
        placed(ONE, [15.6, 16.15]),
    ),
    case(
        "3 x 1.5 corners at 2x",
        rect([10.0, 6.0], [1.5, 0.75]),
        Some(0.75),
        placed(TWO, CENTRE),
    ),
    case(
        "8:1 ellipse filled",
        ellipse([12.0, 1.5]),
        None,
        placed(ONE, CENTRE),
    ),
    case(
        "8:1 ellipse, stroke 4",
        ellipse([12.0, 1.5]),
        Some(4.0),
        placed(ONE, [16.3, 16.2]),
    ),
    case(
        "1:8 ellipse, stroke 4",
        ellipse([1.5, 12.0]),
        Some(4.0),
        placed(ONE, CENTRE),
    ),
    case(
        "4:1 corners rotated, stroke 2",
        rect([16.0, 10.0], [6.0, 1.5]),
        Some(2.0),
        placed(ROTATED, CENTRE),
    ),
    case(
        "4:1 corners skewed, stroke 2",
        rect([12.0, 10.0], [4.0, 1.0]),
        Some(2.0),
        placed(SKEWED, CENTRE),
    ),
    case(
        "4:1 ellipse skewed, stroke 2",
        ellipse([6.0, 1.5]),
        Some(2.0),
        placed(SKEWED, CENTRE),
    ),
];

/// The coverage the exact distance gives the pixel centred at `pixel`.
fn expected(case: &Case, pixel: [f64; 2]) -> f32 {
    let m = case.transform.map(f64::from);
    let det = m[0] * m[3] - m[1] * m[2];
    // device pixels to shape units: the inverse of [a c; b d], by column
    let inverse = [m[3] / det, -m[1] / det, -m[2] / det, m[0] / det];
    let from = [pixel[0] - m[4], pixel[1] - m[5]];
    let local = [
        inverse[0] * from[0] + inverse[2] * from[1],
        inverse[1] * from[0] + inverse[3] * from[1],
    ];
    let (distance, normal) = exact_forms::exact(&case.geometry, local);
    let along = |column: usize| normal[0] * inverse[column] + normal[1] * inverse[column + 1];
    let per_pixel = along(0).hypot(along(2));
    let outside = (case.stroke).map_or(distance, |width| distance.abs() - f64::from(width) / 2.0);
    #[allow(clippy::cast_possible_truncation)]
    let coverage = (0.5 - outside / per_pixel).clamp(0.0, 1.0) as f32;
    coverage
}

/// The largest error over the target, and the pixel it is at.
fn worst(case: &Case) -> (f32, (u32, u32)) {
    let drawn = coverage(&[white(&case.geometry, case.stroke, case.transform)]);
    let mut worst = (0.0, (0, 0));
    for y in 0..SIDE {
        for x in 0..SIDE {
            let off = (drawn[(y * SIDE + x) as usize]
                - expected(case, [f64::from(x) + 0.5, f64::from(y) + 0.5]))
            .abs();
            if off > worst.0 {
                worst = (off, (x, y));
            }
        }
    }
    worst
}

#[test]
fn gpu_elliptical_corners_and_ellipses_are_within_a_twentieth_of_a_pixel_of_exact() {
    // When
    let failed: Vec<String> = CASES
        .iter()
        .map(|case| (case.name, worst(case)))
        .filter(|(_, (off, _))| *off > BOUND)
        .map(|(name, (off, at))| format!("{name}: {off} off at {at:?}"))
        .collect();

    // Then
    assert!(failed.is_empty(), "past {BOUND}:\n{}", failed.join("\n"));
}
