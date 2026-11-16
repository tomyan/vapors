//! A second oracle for shapes on a plane, sharing no code with the GPU
//! stages: each pixel's coverage as 0.5 minus its distance on the screen
//! to the shape's outline, the outline sampled densely in shape units and
//! projected point by point, so no Jacobian is involved. Where the
//! pixel lies within half a device pixel of the outline, it is what the
//! shader's ramp claims to compute.

// each test crate including this uses a part of it
#![allow(dead_code)]

use glam::Mat4;

use crate::scene::{on_screen, pixel_centre, shape_point};

/// A closed outline in shape units, sampled densely enough that its
/// chords lie within 1e-4 of the curve at the tests' scales.
pub type Outline = Vec<[f32; 2]>;

/// A circle about `centre` of `radius`, 4096 points.
pub fn circle_outline(centre: [f32; 2], radius: f32) -> Outline {
    (0..4096u16)
        .map(|k| {
            let angle = f32::from(k) / 4096.0 * std::f32::consts::TAU;
            [
                centre[0] + radius * angle.cos(),
                centre[1] + radius * angle.sin(),
            ]
        })
        .collect()
}

/// A rect at `origin` of `size` with corners of `radius`, grown by
/// `offset` all round (its corners' radius grown alike), 1024 points a
/// corner and 1024 a side.
pub fn rounded_rect_outline(origin: [f32; 2], size: [f32; 2], radius: f32, offset: f32) -> Outline {
    let r = radius + offset;
    let (x0, y0) = (origin[0] + radius, origin[1] + radius);
    let (x1, y1) = (origin[0] + size[0] - radius, origin[1] + size[1] - radius);
    let corners = [[x1, y1, 0.0], [x0, y1, 1.0], [x0, y0, 2.0], [x1, y0, 3.0]];
    corners
        .iter()
        .flat_map(|&[cx, cy, quarter]| {
            (0..1024u16).map(move |k| {
                let angle = (quarter + f32::from(k) / 1023.0) * std::f32::consts::FRAC_PI_2;
                [cx + r * angle.cos(), cy + r * angle.sin()]
            })
        })
        .collect()
}

/// For each pixel of a `side` square target: `Some(expected coverage)`
/// where its centre lies within half a device pixel of any of
/// `outlines` (drawn under `affine` on the plane `camera` sees), else
/// `None`; inside or out is `inside`'s answer at the centre's shape point.
pub fn ramp_by_distance(
    side: u32,
    (outlines, inside): (&[Outline], &dyn Fn([f32; 2]) -> bool),
    affine: [f32; 6],
    camera: (Mat4, Mat4),
) -> Vec<Option<f32>> {
    let place = |q: [f32; 2]| {
        on_screen(
            side,
            camera,
            [
                affine[0] * q[0] + affine[2] * q[1] + affine[4],
                affine[1] * q[0] + affine[3] * q[1] + affine[5],
            ],
        )
    };
    let projected: Vec<Vec<[f32; 2]>> = outlines
        .iter()
        .map(|o| o.iter().map(|q| place(*q)).collect())
        .collect();
    (0..(side * side) as usize)
        .map(|at| {
            let centre = pixel_centre(at, side);
            let distance = projected
                .iter()
                .map(|o| to_polyline(o, centre))
                .fold(f32::INFINITY, f32::min);
            let shape = shape_point(side, affine, camera, centre)?;
            let signed = if inside(shape) { -distance } else { distance };
            (distance <= 0.5).then_some(0.5 - signed)
        })
        .collect()
}

fn to_polyline(points: &[[f32; 2]], p: [f32; 2]) -> f32 {
    let next = points.iter().cycle().skip(1);
    points
        .iter()
        .zip(next)
        .map(|(a, b)| segment_distance(p, *a, *b))
        .fold(f32::INFINITY, f32::min)
}

fn segment_distance(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy).max(1e-12))
        .clamp(0.0, 1.0);
    (a[0] + t * dx - p[0]).hypot(a[1] + t * dy - p[1])
}
