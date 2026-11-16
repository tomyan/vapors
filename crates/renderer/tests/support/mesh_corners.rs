//! The outline corners the band check (`mesh_oracle.rs`) reads, on the
//! device: which are sharp, which places beside them are left to the
//! half-pixel rule, and which inside pixels a triangle without the near
//! corner can cover.

use vapors_shapes::MeshTriangle;

use crate::support::{Place, contains, segment_distance};

use super::SHARP_TURN;

/// An outline corner on the device: its position, its two edges' far
/// ends, the angle between its normals, and whether it is convex.
type Corner = ([f32; 2], [[f32; 2]; 2], f32, bool);

/// A mesh's outline corners on the device.
pub(super) struct Corners {
    pub(super) all: Vec<Corner>,
}

impl Corners {
    /// Each outline corner on the device, its angles taken there: under
    /// a non-uniform scale or a skew, a corner's turn on the device is
    /// not its turn in mesh units, and the shader's is the device's.
    pub(super) fn of(mesh: &[MeshTriangle], place: Place<'_>) -> Self {
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

    pub(super) fn near_sharp(&self, p: [f32; 2]) -> bool {
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
    pub(super) fn left_to_the_half_pixel_rule(&self, p: [f32; 2]) -> bool {
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
pub(super) fn holds_the_corner(
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
