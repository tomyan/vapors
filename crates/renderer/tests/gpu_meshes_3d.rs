//! Mesh items on a plane in a scene pass (docs/plans/decal.md D2): under
//! a camera that maps plane units to device pixels they draw as the page
//! draws them; under a perspective they are as close to an 8x drawing
//! as the page is, and keep the page's rule that every pixel half a
//! device pixel inside the outline is covered and none half a pixel
//! outside is.

#[path = "support/meshes.rs"]
mod page;
use page as support;
#[path = "support/projective.rs"]
mod scene;
#[path = "support/projective_meshes.rs"]
mod scene_meshes;

use glam::{Mat4, Vec3};
use page::{SIDE, assert_ramp_within_half_a_pixel_placed, coverage, mesh, white};
use scene::{on_screen, pixel_camera};
use scene_meshes::{cpu_mesh_coverage, mesh_coverage_3d};

#[path = "support/mesh_oracle.rs"]
mod oracle;

/// The notched rect of `gpu_mesh_insides.rs`, whose corner lines reach
/// past their edges' ends into the fill, and two of touche's strokes, one
/// subpath each (overlapping subpaths blend ramps, S3b's known limit).
fn shapes() -> [(&'static str, Option<f32>); 3] {
    [
        ("M0 0H1V1H2V0H3V3H2.1H0Z", None),
        ("M2 6h7a4 4 0 0 1 0 8H6", Some(1.5)),
        ("M3 10l5-5 5 5", Some(1.5)),
    ]
}

#[test]
fn gpu_a_plane_facing_the_camera_at_one_unit_a_pixel_draws_meshes_as_the_page_does() {
    let notched = [8.0, 0.0, 0.0, 8.0, 4.25, 2.25];
    let turned = [1.6, 1.2, -1.2, 1.6, 14.0, 2.0];
    for (data, stroke) in shapes() {
        for transform in [notched, turned] {
            // Given
            let drawn = [white(mesh(data, stroke), transform)];

            // When
            let scene = mesh_coverage_3d(SIDE, &drawn, (pixel_camera(SIDE), Mat4::IDENTITY));

            // Then
            let page = coverage(&drawn);
            let worst = page
                .iter()
                .zip(&scene)
                .map(|(p, s)| (p - s).abs())
                .fold(0.0, f32::max);
            assert!(
                worst <= 1.0 / 255.0,
                "{data} under {transform:?}: a pixel differs by {worst}"
            );
        }
    }
}

/// How far a band pixel on the turned plane may be from 0.5 minus its
/// distance to the outline: the fragment's map from the derivatives
/// across a 2x2 quad is the local Jacobian to second order, and the
/// worst measured here on 2026-11-16 was 0.029; with the readback's 8
/// bits, 0.032 (the page's own bound is 0.01).
const SCENE_DISTANCE: f32 = 0.032;

/// A 45 degree perspective, square, and a plane 3 units away turned 60
/// degrees about X, the scene's own matrices.
fn turned_plane() -> (Mat4, Mat4) {
    let lens = vapors_scene3d::Projection::perspective(45f32.to_radians(), 0.1, 100.0)
        .and_then(|lens| lens.matrix(1.0))
        .expect("a valid lens");
    let world = Mat4::from_translation(Vec3::new(0.0, 0.0, -3.0))
        * Mat4::from_rotation_x(60f32.to_radians());
    (lens, world)
}

/// Shape units to plane units: 1/8 a unit each, centred (about 12.9
/// device pixels a plane unit there, so a 1.5 stroke is over two device
/// pixels across and one along the turn).
const ON_PLANE: [f32; 6] = [0.125, 0.0, 0.0, 0.125, -1.0, -0.9];

/// `data`'s mesh on the plane, as `ON_PLANE` (scaled up 4 for a fill)
/// places it: its triangles and where a shape unit lands on the screen.
fn on_plane(data: &str, stroke: Option<f32>) -> (Vec<vapors_shapes::MeshTriangle>, [f32; 6]) {
    let scale = if stroke.is_some() { 1.0 } else { 4.0 };
    let plane = [
        ON_PLANE[0] * scale,
        0.0,
        0.0,
        ON_PLANE[3] * scale,
        ON_PLANE[4],
        ON_PLANE[5],
    ];
    (mesh(data, stroke), plane)
}

/// Shape point `q` on the screen under `plane` and the turned camera.
fn placed(plane: [f32; 6], q: [f32; 2]) -> [f32; 2] {
    let on = [
        plane[0] * q[0] + plane[2] * q[1] + plane[4],
        plane[1] * q[0] + plane[3] * q[1] + plane[5],
    ];
    on_screen(SIDE, turned_plane(), on)
}

#[test]
fn gpu_meshes_on_a_plane_at_60_degrees_are_where_a_cpu_drawing_puts_them() {
    for (data, stroke) in shapes() {
        // Given
        let (triangles, plane) = on_plane(data, stroke);

        // When it is drawn, and drawn on the CPU from its own triangles
        let drawn = mesh_coverage_3d(SIDE, &[white(triangles.clone(), plane)], turned_plane());
        let oracle = cpu_mesh_coverage(SIDE, &triangles, plane, turned_plane());

        // Then the page's band rule (support/mesh_oracle.rs), its
        // distances and corners taken on the screen as the camera
        // projects the mesh: every band pixel within SCENE_DISTANCE of
        // 0.5 minus its distance to the outline, and away from sharp
        // corners within 0.11 of the CPU drawing
        oracle::assert_band_matches(
            &drawn,
            &triangles,
            (&|q| placed(plane, q), SCENE_DISTANCE),
            &oracle,
            data,
        );
        let (area, expected) = (drawn.iter().sum::<f32>(), oracle.iter().sum::<f32>());
        // measured on 2026-11-16: area within 1.0% (the notched rect,
        // its sharp corners' ramps)
        assert!(
            (area - expected).abs() <= expected * 0.015,
            "{data}: area {area}, the CPU drawing's {expected}"
        );
    }
}

#[test]
fn gpu_meshes_on_a_plane_at_60_degrees_are_full_inside_and_empty_outside() {
    for (data, stroke) in shapes() {
        // Given
        let (triangles, plane) = on_plane(data, stroke);

        // When
        let drawn = mesh_coverage_3d(SIDE, &[white(triangles.clone(), plane)], turned_plane());

        // Then, each corner placed on the screen as the camera projects it
        assert_ramp_within_half_a_pixel_placed(&drawn, &triangles, &|q| placed(plane, q), data);
    }
}

#[test]
fn gpu_a_mesh_reaching_behind_or_to_the_camera_draws_the_part_in_front() {
    let world = Mat4::from_translation(Vec3::new(0.0, -1.0, 0.0))
        * Mat4::from_rotation_x(-90f32.to_radians());
    for (what, data) in [
        ("from behind the eye", "M-50 -5H50V50H-50Z"),
        ("from the eye's own plane", "M-50 0H50V50H-50Z"),
    ] {
        // When
        let drawn = mesh_coverage_3d(
            SIDE,
            &[white(mesh(data, None), [1.0, 0.0, 0.0, 1.0, 0.0, 0.0])],
            (turned_plane().0, world),
        );

        // Then the bottom quarter is ground, the top quarter sky
        let row = |y: u32| &drawn[(y * SIDE) as usize..((y + 1) * SIDE) as usize];
        for y in 0..SIDE / 4 {
            let below = SIDE - 1 - y;
            assert!(
                row(y).iter().all(|c| *c == 0.0),
                "{what}: row {y} is sky: {:?}",
                row(y)
            );
            assert!(
                row(below).iter().all(|c| *c >= 1.0),
                "{what}: row {below} is ground: {:?}",
                row(below)
            );
        }
    }
}
