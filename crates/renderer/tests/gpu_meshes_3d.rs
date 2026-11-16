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

/// touche's ten icons, as `gpu_mesh_insides.rs` draws them: S3b's set,
/// whose arrowheads take the miter limit's branch of the corner move.
const ICONS: [&str; 10] = [
    "M1.5 4.5v8h13v-7h-6l-1.5-1.5h-5.5z",
    "M5 3H3v11h10V3h-2M5.5 2h5v2.5h-5z",
    "M8 10V2M5 5l3-3 3 3M3 9v5h10V9",
    "M5 3L2 6l3 3M2 6h7a4 4 0 0 1 0 8H6",
    "M11 3l3 3-3 3M14 6H7a4 4 0 0 0 0 8h3",
    "M2 4h12M2 8h12M2 12h12M5 2v4M11 6v4M7 10v4",
    "M3 10l5-5 5 5",
    "M2 6v4M5 4v8M8 2v12M11 4v8M14 6v4",
    "M2 6h3l4-3v10l-4-3H2zM11 6l3 4M14 6l-3 4",
    "M2 8h11M9 4l4 4-4 4",
];

#[test]
fn gpu_a_plane_facing_the_camera_at_one_unit_a_pixel_draws_meshes_as_the_page_does() {
    let doubled = [2.0, 0.0, 0.0, 2.0, 0.0, 0.0];
    let turned = [1.6, 1.2, -1.2, 1.6, 14.0, 2.0];
    let notched = (shapes()[0].0, None, [8.0, 0.0, 0.0, 8.0, 4.25, 2.25]);
    let icons = ICONS
        .iter()
        .flat_map(|data| [(*data, Some(1.5), doubled), (*data, Some(1.5), turned)]);
    for (data, stroke, transform) in icons.chain([notched]) {
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

/// How far a band pixel on a turned plane may be from 0.5 minus its
/// distance to the outline. The fragment's Jacobian is analytic, so what
/// is left is the line distance taken through the Jacobian at the
/// fragment, exact to first order across the half-pixel band, and the
/// readback's 8 bits: measured at most 0.0103 at 60 degrees on
/// 2026-11-16. The page's own bound is 0.01.
const SCENE_DISTANCE: f32 = 0.015;

/// A plane turned `degrees` about X, `distance` from the eye, seen from
/// its back when `behind` (turned a further half turn about Y), through
/// the scene's own 45 degree perspective.
fn camera(degrees: f32, distance: f32, behind: bool) -> (Mat4, Mat4) {
    let lens = vapors_scene3d::Projection::perspective(45f32.to_radians(), 0.1, 100.0)
        .and_then(|lens| lens.matrix(1.0))
        .expect("a valid lens");
    let flip = if behind { std::f32::consts::PI } else { 0.0 };
    let world = Mat4::from_translation(Vec3::new(0.0, 0.0, -distance))
        * Mat4::from_rotation_x(degrees.to_radians())
        * Mat4::from_rotation_y(flip);
    (lens, world)
}

/// The poses the band and insides rules are held at: 60 degrees, and the
/// same seen from behind (the Jacobian's determinant negative).
fn poses() -> [(&'static str, (Mat4, Mat4)); 2] {
    [
        ("60 degrees", camera(60.0, 3.0, false)),
        ("60 degrees from behind", camera(60.0, 3.0, true)),
    ]
}

/// Shape units to plane units: 1/8 a unit each, centred (about 12.9
/// device pixels a plane unit at 60 degrees and 3 units, so a 1.5 stroke
/// is over two device pixels across and one along the turn).
const ON_PLANE: [f32; 6] = [0.125, 0.0, 0.0, 0.125, -1.0, -0.9];

/// `data`'s mesh on the plane, as `ON_PLANE` (scaled up 4 for a fill)
/// places it: its triangles and where a shape unit lands on the plane.
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

/// Shape point `q` on the screen under `plane` and `camera`.
fn placed(plane: [f32; 6], camera: (Mat4, Mat4), q: [f32; 2]) -> [f32; 2] {
    let on = [
        plane[0] * q[0] + plane[2] * q[1] + plane[4],
        plane[1] * q[0] + plane[3] * q[1] + plane[5],
    ];
    on_screen(SIDE, camera, on)
}

#[test]
fn gpu_meshes_on_turned_planes_are_where_a_cpu_drawing_puts_them() {
    for ((data, stroke), (pose, camera)) in shapes()
        .into_iter()
        .flat_map(|s| poses().map(move |p| (s, p)))
    {
        // Given
        let (triangles, plane) = on_plane(data, stroke);
        let what = format!("{data} at {pose}");

        // When it is drawn, and drawn on the CPU from its own triangles
        let drawn = mesh_coverage_3d(SIDE, &[white(triangles.clone(), plane)], camera);
        let oracle = cpu_mesh_coverage(SIDE, &triangles, plane, camera);

        // Then the page's band rule (support/mesh_oracle.rs), its
        // distances and corners taken on the screen as the camera
        // projects the mesh
        let project = |q| placed(plane, camera, q);
        oracle::assert_band_matches(
            &drawn,
            &triangles,
            (&project, SCENE_DISTANCE),
            &oracle,
            &what,
        );
    }
}

#[test]
fn gpu_meshes_on_turned_planes_are_full_inside_and_empty_outside() {
    for ((data, stroke), (pose, camera)) in shapes()
        .into_iter()
        .flat_map(|s| poses().map(move |p| (s, p)))
    {
        // Given
        let (triangles, plane) = on_plane(data, stroke);

        // When
        let drawn = mesh_coverage_3d(SIDE, &[white(triangles.clone(), plane)], camera);

        // Then, each corner placed on the screen as the camera projects it
        let project = |q| placed(plane, camera, q);
        assert_ramp_within_half_a_pixel_placed(
            &drawn,
            &triangles,
            &project,
            &format!("{data} at {pose}"),
        );
    }
}

/// Ground under an eye 1 unit up, from plane y `near` to 50 units ahead.
fn ground_from(near: f32) -> Vec<f32> {
    let data = format!("M-50 {near}H50V50H-50Z");
    let world = Mat4::from_translation(Vec3::new(0.0, -1.0, 0.0))
        * Mat4::from_rotation_x(-90f32.to_radians());
    let identity = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    mesh_coverage_3d(
        SIDE,
        &[white(mesh(&data, None), identity)],
        (camera(0.0, 0.0, false).0, world),
    )
}

#[test]
fn gpu_a_mesh_reaching_behind_or_to_the_camera_draws_the_part_in_front() {
    for (what, near) in [
        ("from behind the eye", -5.0),
        ("from the eye's own plane", 0.0),
    ] {
        // When
        let drawn = ground_from(near);

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
