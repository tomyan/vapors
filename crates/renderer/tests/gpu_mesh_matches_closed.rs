//! GPU tier: a mesh's corners are exact (S3a3, #371 review): a rect
//! rotated 30 degrees drawn as a mesh matches the closed form pixel for
//! pixel, the pixels just beyond its corners included, which take the
//! device distance to the corner as the closed form does.

#[path = "support/closed_forms.rs"]
mod closed;
#[path = "support/meshes.rs"]
mod support;

use support::{coverage, mesh, white};

#[test]
fn gpu_a_rotated_rect_matches_the_closed_form() {
    // Given: a 12 x 8 rect rotated 30 degrees, as a mesh and as the
    // closed form
    let (sin, cos) = 30f32.to_radians().sin_cos();
    let transform = [cos, sin, -sin, cos, 16.0, 16.0];
    let rect = closed::rect([-6.0, -4.0], [12.0, 8.0], 0.0);

    // When
    let meshed = coverage(&[white(mesh("M-6 -4h12v8h-12z", None), transform)]);
    let exact = closed::coverage(&[closed::white(&rect, None, transform)]);

    // Then: every pixel within two 8-bit steps
    for (at, (m, c)) in (0u32..).zip(meshed.iter().zip(&exact)) {
        let pixel = (at % 32, at / 32);
        assert!(
            (m - c).abs() <= 2.0 / 255.0,
            "{pixel:?}: mesh {m}, closed form {c}"
        );
    }
}
