//! An oracle for the mesh pipeline's antialiasing ramp that shares no
//! code with the shader: each pixel's coverage drawn on the CPU from the
//! mesh's own triangles (8 x 8 samples a pixel, a sample covered when a
//! triangle under the transform contains it), and the rule that every
//! pixel within half a device pixel of the outline (the ramp's band,
//! which `support::assert_ramp_within_half_a_pixel` does not judge) is
//! within a bound of it; within a device pixel of a sharp corner, where
//! a ramp and a box filter part (a pixel centred on a right-angled
//! corner: 0.5 against 0.25), within a tight bound of 0.5 minus its CPU
//! distance to the outline instead.

// each test crate including this uses a part of it
#![allow(dead_code)]

use vapors_shapes::MeshTriangle;

#[path = "mesh_corners.rs"]
mod corners;
use corners::{Corners, holds_the_corner};

use crate::support::{Place, SIDE, contains, inside_by, outside_by, pixel_centres, to_device};

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

/// How to run the tests that use this oracle.
const REPRODUCE: &str = "reproduce: cargo nextest run -p vapors-renderer -E 'binary(gpu_mesh_insides) | binary(gpu_mesh_corner_sweep) | binary(gpu_meshes_3d)'";

/// The most a band pixel may differ from the CPU drawing: a ramp of
/// 1/2 - d differs from a box filter by up to 0.043 on a straight edge
/// (at 45 degrees the box is (1/sqrt 2 - d)^2, and |d^2 - (sqrt 2 - 1) d|
/// peaks at 0.043 on [0, 1/2]); 8 x 8 samples misplace an edge's area by
/// up to half a sample column, 1/16; and the readback's 8 bits by 0.002.
/// 0.108 in all.
pub const BAND_TOLERANCE: f32 = 0.11;

/// A corner turning more than this (radians between its two outline
/// normals) is sharp, and within a device pixel of it a pixel is judged
/// by its distance instead of the box filter: a pixel centred on a
/// corner turning t differs from a box filter by about t / 2 pi, and the
/// band's budget leaves 0.11 - 1/16 - 0.002 = 0.046 for it, t = 0.28.
pub const SHARP_TURN: f32 = 0.28;

/// How far a pixel may be from 0.5 minus its CPU distance to the
/// outline: the readback's 8 bits (0.002) and the shader's f32
/// arithmetic. Measured on the cases of `gpu_mesh_insides.rs` with
/// #420's fix: at most 0.0033.
pub const DISTANCE_TOLERANCE: f32 = 0.01;

/// How far under 0.5 minus its distance an inside pixel may read where a
/// triangle without the near sharp corner can cover it (#421): measured
/// at most 0.0124 under, on Redo at 1.55 (pixel (22, 15)), on 2026-11-16;
/// that and one more 8-bit step's margin, 0.02.
pub const UNDER_TOLERANCE: f32 = 0.02;

/// Asserts every pixel whose centre lies within half a device pixel of
/// `mesh`'s outline under `transform` is within [`DISTANCE_TOLERANCE`]
/// of 0.5 minus its CPU distance to the outline, which is what the
/// shader claims to compute, and, away from sharp corners, within
/// [`BAND_TOLERANCE`] of the CPU drawing. The places left to the
/// half-pixel rule are each named where they are tested. Refuses a check
/// that judged no pixel at all, naming `what`.
pub fn assert_band_matches_cpu(
    coverage: &[f32],
    mesh: &[MeshTriangle],
    transform: [f32; 6],
    what: &str,
) {
    let drawing = cpu_coverage(mesh, transform);
    assert_band_matches(
        coverage,
        mesh,
        (
            &|q| to_device(transform, q),
            DISTANCE_TOLERANCE,
            BAND_TOLERANCE,
        ),
        &drawing,
        what,
    );
}

/// [`assert_band_matches_cpu`] with the corners placed by `place` (a map
/// that keeps lines straight), a pixel's distance judged within
/// `tolerance` and its box-filtered value within `box_tolerance`, and
/// `drawing` the CPU drawing under `place`.
pub fn assert_band_matches(
    coverage: &[f32],
    mesh: &[MeshTriangle],
    (place, tolerance, box_tolerance): (Place<'_>, f32, f32),
    drawing: &[f32],
    what: &str,
) {
    let band = Band {
        mesh,
        place,
        tolerance,
        box_tolerance,
        corners: Corners::of(mesh, place),
        drawing,
    };
    let mut judged = 0;
    for (x, y, centre) in pixel_centres() {
        let drawn = coverage[(y * SIDE + x) as usize];
        if let Some(failure) = band.judge(centre, drawn, (y * SIDE + x) as usize) {
            panic!(
                "{what}: pixel ({x}, {y}) covered {drawn}; {failure}; fix: the mesh shader's \
                 wedge distances (meshes_shared.wgsl, `wedge`) or the bound's source; {REPRODUCE}"
            );
        }
        judged += usize::from(band.judged(centre));
    }
    assert!(
        judged > 0,
        "{what}: the band check judged no pixel; fix: pass the placement the mesh was drawn \
         under; {REPRODUCE}"
    );
}

/// What the band check reads for one mesh.
struct Band<'a> {
    mesh: &'a [MeshTriangle],
    place: Place<'a>,
    tolerance: f32,
    box_tolerance: f32,
    corners: Corners,
    drawing: &'a [f32],
}

impl Band<'_> {
    /// Whether the pixel at `centre` is in the band and not left to the
    /// half-pixel rule.
    fn judged(&self, centre: [f32; 2]) -> bool {
        let (outside, inside) = self.distances(centre);
        let skipped = self.corners.left_to_the_half_pixel_rule(centre);
        outside <= 0.5 && inside < 0.5 && !skipped
    }

    /// Why the pixel at `centre`, covered `drawn`, fails, if it does.
    fn judge(&self, centre: [f32; 2], drawn: f32, at: usize) -> Option<String> {
        if !self.judged(centre) {
            return None;
        }
        let (outside, inside) = self.distances(centre);
        let by_distance = (0.5 - outside + inside).clamp(0.0, 1.0);
        let own_edges =
            inside == 0.0 || holds_the_corner(self.mesh, self.place, &self.corners, centre);
        if own_edges && (drawn - by_distance).abs() > self.tolerance {
            return Some(format!(
                "0.5 minus its distance to the outline is {by_distance}"
            ));
        }
        // A triangle without the near corner over-covers by up to the
        // corner's distance (it reads a farther edge of its own; #421), so
        // an inside pixel it may cover is held from below only, within
        // UNDER_TOLERANCE, or the caller's bound where that is wider
        if !own_edges && drawn < by_distance - UNDER_TOLERANCE.max(self.tolerance) {
            return Some(format!(
                "under 0.5 minus its distance to the outline, {by_distance}"
            ));
        }
        let boxed = self.drawing[at];
        let near_sharp = self.corners.near_sharp(centre);
        (!near_sharp && (drawn - boxed).abs() > self.box_tolerance)
            .then(|| format!("the CPU drawing has {boxed}"))
    }

    fn distances(&self, centre: [f32; 2]) -> (f32, f32) {
        (
            outside_by(self.mesh, self.place, centre),
            inside_by(self.mesh, self.place, centre),
        )
    }
}
