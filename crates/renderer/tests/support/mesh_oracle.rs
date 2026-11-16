//! An oracle for the mesh pipeline's antialiasing ramp that shares no
//! code with the shader: each pixel's coverage drawn on the CPU from the
//! mesh's own triangles (8 x 8 samples a pixel, a sample covered when a
//! triangle under the transform contains it), and the rule that every
//! pixel within half a device pixel of the outline (the ramp's band,
//! which `support::assert_ramp_within_half_a_pixel` does not judge) is
//! within a bound of it. Pixels within a device pixel of a sharp corner
//! are left to that rule, as S3b measured corners against Chromium.

// each test crate including this uses a part of it
#![allow(dead_code)]

use vapors_shapes::MeshTriangle;

use crate::support::{SIDE, contains, inside_by, outside_by, pixel_centres, to_device};

/// Each pixel's coverage of `mesh` under `transform`, drawn on the CPU.
pub fn cpu_coverage(mesh: &[MeshTriangle], transform: [f32; 6]) -> Vec<f32> {
    let triangles: Vec<[[f32; 2]; 3]> = mesh
        .iter()
        .map(|t| t.corners.map(|c| to_device(transform, c.position)))
        .collect();
    let covered = |p: [f32; 2]| triangles.iter().any(|t| contains(*t, p));
    pixel_centres()
        .map(|(_, _, [cx, cy])| {
            let offset = |k: u16| (f32::from(k) + 0.5) / 8.0 - 0.5;
            let hits = (0..64u16)
                .filter(|k| covered([cx + offset(k % 8), cy + offset(k / 8)]))
                .count();
            f32::from(u16::try_from(hits).expect("at most 64")) / 64.0
        })
        .collect()
}

/// The most a band pixel may differ from the CPU drawing: a linear ramp
/// differs from a box filter by up to 0.043 on a straight edge (at 45
/// degrees: (1/sqrt 2 - d)^2 against 1/2 - sqrt 2 d), and 8 x 8 samples
/// misplace an edge's area by up to one sample column, 1/16.
pub const BAND_TOLERANCE: f32 = 0.11;

/// A corner turning more than this (radians between its two outline
/// normals) is sharp: within a device pixel of it, a ramp and a box
/// filter differ by up to a quarter (a pixel centred on a right-angled
/// corner: 0.5 against 0.25), which the half-pixel rule judges instead.
pub const SHARP_TURN: f32 = 0.35;

/// Asserts every pixel whose centre lies within half a device pixel of
/// `mesh`'s outline under `transform`, away from the places the module
/// says, is within [`BAND_TOLERANCE`] of the CPU drawing.
pub fn assert_band_matches_cpu(
    coverage: &[f32],
    mesh: &[MeshTriangle],
    transform: [f32; 6],
    what: &str,
) {
    let oracle = cpu_coverage(mesh, transform);
    for (x, y, centre) in pixel_centres() {
        let at = (y * SIDE + x) as usize;
        let (drawn, expected) = (coverage[at], oracle[at]);
        if (drawn - expected).abs() <= BAND_TOLERANCE || !in_band(mesh, transform, centre) {
            continue;
        }
        assert!(
            left_to_the_half_pixel_rule(mesh, transform, centre),
            "{what}: pixel ({x}, {y}), in the ramp's band, covered {drawn}; the CPU drawing has {expected}"
        );
    }
}

fn in_band(mesh: &[MeshTriangle], transform: [f32; 6], centre: [f32; 2]) -> bool {
    outside_by(mesh, transform, centre) <= 0.5 && inside_by(mesh, transform, centre) < 0.5
}

/// Within a device pixel of a sharp corner.
fn left_to_the_half_pixel_rule(
    mesh: &[MeshTriangle],
    transform: [f32; 6],
    centre: [f32; 2],
) -> bool {
    let place = |q| to_device(transform, q);
    let near = |q: [f32; 2]| {
        let c = place(q);
        (c[0] - centre[0]).hypot(c[1] - centre[1]) < 1.0
    };
    sharp_corners(mesh).any(near)
}

fn sharp_corners(mesh: &[MeshTriangle]) -> impl Iterator<Item = [f32; 2]> + '_ {
    mesh.iter()
        .flat_map(|t| t.corners)
        .filter(|c| c.kind != 0.0)
        .filter(|c| {
            let [n0, n1] = c.normals;
            (n0[0] * n1[0] + n0[1] * n1[1]).clamp(-1.0, 1.0).acos() > SHARP_TURN
        })
        .map(|c| c.position)
}
