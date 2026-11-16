//! GPU tier: the mesh ramp beside every kind of convex corner, from a
//! tip of 5 degrees to a corner of 175 (turning 175 to 5), and under a
//! 4:1 scale, held to the CPU oracles of `support/mesh_oracle.rs` (#420's
//! review, M1 and L1): within a pixel of a corner a pixel's coverage is
//! 0.5 minus its distance to the outline, which no box-filter allowance
//! hides.

#[path = "support/mesh_oracle.rs"]
mod oracle;
#[path = "support/meshes.rs"]
mod support;

use support::{coverage, mesh, white};

/// A filled pentagon whose top corner has an interior angle of
/// `degrees`, its sides 12 px long, then down to a base at y = 27.4, so
/// its other corners turn at most 90 degrees; the top corner off the
/// pixel grid.
fn tip(degrees: f32) -> String {
    let half = (degrees / 2.0).to_radians();
    let (dx, dy) = (12.0 * half.sin(), 12.0 * half.cos());
    let (x, y) = (16.3, 4.6);
    let (left, right, side) = (x - dx, x + dx, y + dy);
    format!("M{x} {y}L{right} {side}L{right} 27.4L{left} 27.4L{left} {side}Z")
}

#[test]
fn gpu_the_ramp_beside_convex_corners_of_5_to_175_degrees_is_its_distance() {
    for degrees in (0..18).map(|k| 5.0 + 10.0 * f32::from(u8::try_from(k).expect("small"))) {
        // Given
        let data = tip(degrees);
        let triangles = mesh(&data, None);
        let identity = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

        // When
        let drawn = coverage(&[white(triangles.clone(), identity)]);

        // Then every band pixel is judged, but beside a tip under 29
        // degrees and along its sides, which the miter clamp cuts (#419)
        let what = format!("a {degrees} degree corner");
        support::assert_ramp_within_half_a_pixel(&drawn, &triangles, identity, &what);
        oracle::assert_band_matches_cpu(&drawn, &triangles, identity, &what);
    }
}

#[test]
fn gpu_the_ramp_under_a_4_to_1_scale_is_its_distance_on_the_device() {
    // Given touche's Redo arc subpath stretched 4:1, where the corner's
    // region in mesh units and on the device part by about 37 degrees
    let data = "M2 6h7a4 4 0 0 1 0 8H6";
    let triangles = mesh(data, Some(1.5));
    let stretched = [2.6, 0.0, 0.0, 0.65, -4.0, 0.0];

    // When
    let drawn = coverage(&[white(triangles.clone(), stretched)]);

    // Then
    oracle::assert_band_matches_cpu(&drawn, &triangles, stretched, "Redo's arc at 4:1");
}
