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

use crate::support::{
    Place, SIDE, contains, inside_by, outside_by, pixel_centres, segment_distance, to_device,
};

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
        (&|q| to_device(transform, q), DISTANCE_TOLERANCE),
        &drawing,
        what,
    );
}

/// [`assert_band_matches_cpu`] with the corners placed by `place` (a map
/// that keeps lines straight), a pixel's distance judged within
/// `tolerance`, and `drawing` the CPU drawing under `place`.
pub fn assert_band_matches(
    coverage: &[f32],
    mesh: &[MeshTriangle],
    (place, tolerance): (Place<'_>, f32),
    drawing: &[f32],
    what: &str,
) {
    let band = Band {
        mesh,
        place,
        tolerance,
        corners: Corners::of(mesh, place),
        drawing,
    };
    let mut judged = 0;
    for (x, y, centre) in pixel_centres() {
        let drawn = coverage[(y * SIDE + x) as usize];
        if let Some(failure) = band.judge(centre, drawn, (y * SIDE + x) as usize) {
            panic!("{what}: pixel ({x}, {y}) covered {drawn}; {failure}");
        }
        judged += usize::from(band.judged(centre));
    }
    assert!(
        judged > 0,
        "{what}: the band check judged no pixel: is the placement the one drawn?"
    );
}

/// What the band check reads for one mesh.
struct Band<'a> {
    mesh: &'a [MeshTriangle],
    place: Place<'a>,
    tolerance: f32,
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
        let boxed = self.drawing[at];
        let near_sharp = self.corners.near_sharp(centre);
        (!near_sharp && (drawn - boxed).abs() > BAND_TOLERANCE)
            .then(|| format!("the CPU drawing has {boxed}"))
    }

    fn distances(&self, centre: [f32; 2]) -> (f32, f32) {
        (
            outside_by(self.mesh, self.place, centre),
            inside_by(self.mesh, self.place, centre),
        )
    }
}

/// An outline corner on the device: its position, its two edges' far
/// ends, the angle between its normals, and whether it is convex.
type Corner = ([f32; 2], [[f32; 2]; 2], f32, bool);

/// A mesh's outline corners on the device.
struct Corners {
    all: Vec<Corner>,
}

impl Corners {
    /// Each outline corner on the device, its angles taken there: under
    /// a non-uniform scale or a skew, a corner's turn on the device is
    /// not its turn in mesh units, and the shader's is the device's.
    fn of(mesh: &[MeshTriangle], place: Place<'_>) -> Self {
        let all = mesh
            .iter()
            .flat_map(|t| t.corners)
            .filter(|c| c.kind != 0.0)
            .map(|c| {
                let end = |k: usize| {
                    let (n, reach) = (c.normals[k], c.reaches[k]);
                    place([c.position[0] - n[1] * reach, c.position[1] + n[0] * reach])
                };
                let (at, ends) = (place(c.position), [end(0), end(1)]);
                (at, ends, between_normals(at, ends), c.kind > 0.0)
            })
            .collect();
        Self { all }
    }

    fn near_sharp(&self, p: [f32; 2]) -> bool {
        self.all
            .iter()
            .any(|(c, _, between, _)| *between > SHARP_TURN && within_a_pixel(*c, p))
    }

    /// RELAXED (#419): beside a corner whose normals are more than 151
    /// degrees apart, the vertex stage's move is capped at the miter
    /// limit, its edges sit under half a pixel out, and the outer ramp
    /// is cut along them: pixels within a pixel of the corner or of
    /// either of its edges, until #419 is fixed. And by design (meshes.wgsl,
    /// `wedge`): past an acute convex tip, behind both its edges, the
    /// shader keeps a line's distance, the tip mitred, not the distance
    /// to the tip a CPU distance gives.
    fn left_to_the_half_pixel_rule(&self, p: [f32; 2]) -> bool {
        self.all.iter().any(|(c, ends, between, convex)| {
            let beside =
                within_a_pixel(*c, p) || ends.iter().any(|e| segment_distance(p, *c, *e) <= 1.0);
            let clamped = (between / 2.0).cos() < 0.25 && beside;
            let past_a_tip =
                *convex && *between > std::f32::consts::FRAC_PI_2 && behind_both(*c, *ends, p);
            clamped || past_a_tip
        })
    }
}

fn within_a_pixel(c: [f32; 2], p: [f32; 2]) -> bool {
    (c[0] - p[0]).hypot(c[1] - p[1]) < 1.0
}

fn behind_both(c: [f32; 2], ends: [[f32; 2]; 2], p: [f32; 2]) -> bool {
    let q = [p[0] - c[0], p[1] - c[1]];
    ends.iter()
        .all(|e| q[0] * (e[0] - c[0]) + q[1] * (e[1] - c[1]) <= 0.0)
}

/// The angle between a corner's outward normals on the device: pi less
/// the angle between its two edges, which run from `at` to `ends`.
fn between_normals(at: [f32; 2], ends: [[f32; 2]; 2]) -> f32 {
    let [a, b] = ends.map(|e| [e[0] - at[0], e[1] - at[1]]);
    let cosine = (a[0] * b[0] + a[1] * b[1]) / (a[0].hypot(a[1]) * b[0].hypot(b[1]));
    std::f32::consts::PI - cosine.clamp(-1.0, 1.0).acos()
}

/// Whether every triangle that can cover `centre` has, among its own
/// corners, each sharp corner within a device pixel of it. A triangle's
/// coverage comes from its own corners' edges only (`MeshCorner`), and
/// the vertex stage's move drags its inner edges by up to half a pixel,
/// so a triangle within a pixel of the centre can cover it; one without
/// the corner reads the nearer of its own edges there and can over-cover
/// by up to the corner's distance. An inside pixel where that can happen
/// is left to the half-pixel rule, which judges those deeper than half
/// a pixel (the review's fan in `gpu_mesh_insides.rs`: pixel (20, 10),
/// 0.35 px inside the notch's corner, reads 1).
fn holds_the_corner(
    mesh: &[MeshTriangle],
    place: Place<'_>,
    corners: &Corners,
    centre: [f32; 2],
) -> bool {
    let near: Vec<[f32; 2]> = corners
        .all
        .iter()
        .filter(|(c, _, between, _)| *between > SHARP_TURN && within_a_pixel(*c, centre))
        .map(|(c, ..)| *c)
        .collect();
    mesh.iter()
        .map(|t| t.corners.map(|c| place(c.position)))
        .filter(|t| {
            contains(*t, centre)
                || t.iter()
                    .zip(t.iter().cycle().skip(1))
                    .any(|(a, b)| segment_distance(centre, *a, *b) <= 1.0)
        })
        .all(|t| {
            near.iter().all(|c| {
                t.iter()
                    .any(|v| (v[0] - c[0]).abs() < 1e-4 && (v[1] - c[1]).abs() < 1e-4)
            })
        })
}
