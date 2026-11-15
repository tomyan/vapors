//! The classifier's contract, rasterised: for each accepted path, the
//! closed form's coverage is the path's, pixel for pixel at 64 times
//! the viewBox; for the chord short of the diameter, the midpoint
//! ellipse is not, which is why it stays a path.

use zeno::{Fill, Format, Mask, Origin, Transform};

use super::*;
use crate::canonical::FillRule;

/// Pixels per viewBox unit.
const SCALE: f32 = 64.0;
/// Pixels across: a 16-unit viewBox.
const SIDE: u32 = 1024;
/// Subsamples per axis for the analytic coverage.
const SUB: u32 = 4;

/// zeno's coverage of the filled path, one byte per pixel.
fn rasterised(data: &str) -> Vec<u8> {
    let path = IconPath::parse(data).unwrap_or_else(|e| panic!("{data:?}: {e}"));
    Mask::new(path.commands())
        .style(Fill::NonZero)
        .transform(Some(Transform::scale(SCALE, SCALE)))
        .format(Format::Alpha)
        .origin(Origin::TopLeft)
        .size(SIDE, SIDE)
        .render()
        .0
}

/// Whether the viewBox point is inside the closed form.
fn inside(form: Classified, x: f32, y: f32) -> bool {
    match form {
        Classified::Rect {
            origin,
            size,
            radius,
        } => inside_rect(origin, size, radius, [x, y]),
        Classified::Ellipse { centre, radii } => {
            ((x - centre[0]) / radii[0]).powi(2) + ((y - centre[1]) / radii[1]).powi(2) <= 1.0
        }
        Classified::Path => panic!("a path has no closed form"),
    }
}

/// Whether `p` is inside the rect at `origin` of `size` with corners
/// of `radius`: inside the box, and, past a corner's centre on both
/// axes, inside that corner's ellipse.
fn inside_rect(origin: [f32; 2], size: [f32; 2], radius: [f32; 2], p: [f32; 2]) -> bool {
    let from_centre = [0, 1].map(|i| (p[i] - origin[i] - size[i] / 2.0).abs());
    let past = [0, 1].map(|i| from_centre[i] - (size[i] / 2.0 - radius[i]));
    let in_box = (0..2).all(|i| from_centre[i] <= size[i] / 2.0);
    let in_corner = past.iter().all(|v| *v > 0.0);
    in_box && (!in_corner || (past[0] / radius[0]).powi(2) + (past[1] / radius[1]).powi(2) <= 1.0)
}

/// The largest difference, in coverage bytes, between the path
/// rasterised and the closed form sampled `SUB` x `SUB` per pixel.
#[allow(clippy::cast_precision_loss)]
fn mismatch(data: &str, form: Classified) -> u8 {
    let (worst, _) = mismatch_at(data, form);
    worst
}

/// The largest difference and the pixel it is at, with both coverages.
#[allow(clippy::cast_precision_loss)]
fn mismatch_at(data: &str, form: Classified) -> (u8, (u32, u32, u8, u8)) {
    let mask = rasterised(data);
    let mut worst = 0u8;
    let mut at = (0, 0, 0, 0);
    for py in 0..SIDE {
        for px in 0..SIDE {
            let mut hits = 0u32;
            for sy in 0..SUB {
                for sx in 0..SUB {
                    let x = (px as f32 + (sx as f32 + 0.5) / SUB as f32) / SCALE;
                    let y = (py as f32 + (sy as f32 + 0.5) / SUB as f32) / SCALE;
                    hits += u32::from(inside(form, x, y));
                }
            }
            let analytic = u8::try_from(hits * 255 / (SUB * SUB)).expect("at most 255");
            let found = mask[(py * SIDE + px) as usize];
            if analytic.abs_diff(found) > worst {
                worst = analytic.abs_diff(found);
                at = (px, py, analytic, found);
            }
        }
    }
    (worst, at)
}

/// What the reference itself is off by, in coverage bytes: one
/// subsample step (16), zeno's cubic arcs 2.7e-4 r out (0.1 px at this
/// scale, 26) and zeno's flattening of those cubics within about a
/// quarter pixel (64); measured 78 on the circle. The chord short of
/// the diameter below is 1.2 px off at this scale, well past it.
const ALLOWED: u8 = 110;

#[test]
fn each_accepted_closed_form_covers_what_its_path_covers() {
    // Given
    let cases = [
        "M1 1h4v3h-4z",
        "M5 4V1H1V4Z",
        "M2 8a6 6 0 1 0 12 0a6 6 0 1 0-12 0z",
        "M14 8a6 6 0 0 1-12 0a6 6 0 0 1 12 0z",
        "M8 2a6 4 0 1 0 0 8a6 4 0 1 0 0-8z",
        "M3 1H9A2 2 0 0 1 11 3V5A2 2 0 0 1 9 7H3A2 2 0 0 1 1 5V3A2 2 0 0 1 3 1Z",
        "M4 1h4a3 1.5 0 0 1 3 1.5v3a3 1.5 0 0 1-3 1.5h-4a3 1.5 0 0 1-3-1.5v-3a3 1.5 0 0 1 3-1.5z",
    ];

    // When, Then
    for data in cases {
        let path = IconPath::parse(data).expect("parses");
        let form = classify(&path, Paint::Fill(FillRule::NonZero));
        assert_ne!(form, Classified::Path, "{data}");
        let (worst, at) = mismatch_at(data, form);
        assert!(
            worst <= ALLOWED,
            "{data}: {form:?} differs by {worst} at {at:?}"
        );
    }
}

#[test]
fn a_chord_short_of_the_diameter_is_not_the_midpoint_ellipse() {
    // Given: 6e-5 short, the two centres 0.019 apart; the midpoint
    // ellipse the chord test alone would return
    let data = "M2 8A6 6 0 1 0 13.99994 8A6 6 0 1 0 2 8z";
    let midpoint = Classified::Ellipse {
        centre: [7.99997, 8.0],
        radii: [6.0, 6.0],
    };

    // When, Then: the rasteriser shows the difference, and the
    // classifier leaves it a path
    let worst = mismatch(data, midpoint);
    assert!(worst > ALLOWED, "differs by only {worst}");
    let path = IconPath::parse(data).expect("parses");
    assert_eq!(
        classify(&path, Paint::Fill(FillRule::NonZero)),
        Classified::Path
    );
}
