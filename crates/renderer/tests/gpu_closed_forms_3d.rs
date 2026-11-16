//! Closed-form shapes on a plane in a scene pass (docs/plans/decal.md
//! D1): under a camera that maps plane units to device pixels they draw
//! as the page draws them, and under a perspective they keep their
//! edges where an 8x drawing puts them.

#[path = "support/closed_forms.rs"]
mod page;
#[path = "support/projective.rs"]
mod scene;

use glam::{Mat4, Vec3};
use page::{SIDE, circle, coverage, rect, scaled, white};
use scene::{coverage_3d, cpu_coverage_8x, pixel_camera, pixels_3d, rounded_rect_distance};
use vapors_renderer::ClosedFormShape;
use vapors_shapes::Geometry;

/// The shapes the page's tests draw, and a rotated, scaled pair.
fn page_cases() -> Vec<(&'static str, ClosedFormShape)> {
    let turned = Mat4::from_rotation_z(30f32.to_radians()) * Mat4::from_scale(Vec3::splat(1.37));
    let [a, b, _, _, c, d, ..] = turned.to_cols_array();
    let turned = [a, b, c, d, 16.0, 6.0];
    let elliptical = Geometry::Rect {
        origin: [4.0, 6.0],
        size: [22.0, 18.0],
        radius: [7.0, 3.0],
    };
    vec![
        (
            "rect at 1x",
            white(&rect([8.3, 8.3], [12.0, 9.0], 0.0), None, scaled(1.0)),
        ),
        (
            "rect at 2x",
            white(&rect([4.15, 4.15], [6.0, 4.5], 0.0), None, scaled(2.0)),
        ),
        (
            "circle",
            white(&circle([16.0, 16.0], 9.6), None, scaled(1.0)),
        ),
        (
            "rounded rect, stroked",
            white(&rect([5.0, 5.0], [20.0, 16.0], 4.0), Some(1.5), scaled(1.0)),
        ),
        ("elliptical corners", white(&elliptical, None, scaled(1.0))),
        (
            "rotated 30 and scaled 1.37",
            white(&rect([0.0, 0.0], [10.0, 6.0], 2.0), Some(1.0), turned),
        ),
    ]
}

#[test]
fn gpu_a_plane_facing_the_camera_at_one_unit_a_pixel_draws_as_the_page_does() {
    for (what, shape) in page_cases() {
        // Given
        let page = coverage(&[shape]);

        // When
        let scene = coverage_3d(SIDE, &[shape], pixel_camera(SIDE), Mat4::IDENTITY);

        // Then
        let worst = page
            .iter()
            .zip(&scene)
            .map(|(p, s)| (p - s).abs())
            .fold(0.0, f32::max);
        assert!(
            worst <= 1.0 / 255.0,
            "{what}: a pixel differs from the page's by {worst}"
        );
    }
}

/// The scene's perspective: a 45 degree field of view, square.
fn perspective() -> Mat4 {
    vapors_scene3d::Projection::perspective(45f32.to_radians(), 0.1, 100.0)
        .and_then(|lens| lens.matrix(1.0))
        .expect("a valid lens")
}

/// A camera 3 units from a plane turned `degrees` about X, 45 degree
/// field of view.
fn turned_plane(degrees: f32) -> (Mat4, Mat4) {
    let view_projection = perspective();
    let world = Mat4::from_translation(Vec3::new(0.0, 0.0, -3.0))
        * Mat4::from_rotation_x(degrees.to_radians());
    (view_projection, world)
}

/// A circle and a stroked rect with round corners in shape units under
/// `transform`, which a test scales.
fn circle_and_frame(transform: impl Fn(f32) -> [f32; 6], scale: f32) -> Vec<ClosedFormShape> {
    vec![
        white(&circle([-6.0, 4.0], 12.0), None, transform(scale)),
        white(
            &rect([4.0, -14.0], [14.0, 10.0], 3.0),
            Some(4.0),
            transform(scale),
        ),
    ]
}

/// Whether shape point `p` is in `circle_and_frame`'s shapes: the
/// circle, or the stroke 4 wide centred on the rounded rect's outline.
fn in_circle_and_frame(p: [f32; 2]) -> bool {
    let in_circle = (p[0] + 6.0).hypot(p[1] - 4.0) <= 12.0;
    let on_frame = rounded_rect_distance(p, [4.0, -14.0], [14.0, 10.0], 3.0).abs() <= 2.0;
    in_circle || on_frame
}

#[test]
fn gpu_a_plane_at_60_degrees_keeps_its_edges_where_a_cpu_drawing_puts_them() {
    // Given the shapes on a plane turned 60 degrees, plane units of 1/20
    // a shape unit (about 12.9 device pixels a plane unit there)
    let camera = turned_plane(60.0);
    let shapes = circle_and_frame(|_| scaled(0.05), 1.0);

    // When drawn, and on the CPU at 8 x 8 samples a pixel
    let drawn = coverage_3d(SIDE, &shapes, camera.0, camera.1);
    let oracle = cpu_coverage_8x(SIDE, &in_circle_and_frame, scaled(0.05), camera);

    // Then within what was measured on 2026-11-16 (D1's review round):
    // worst pixel 0.153, area 143.59 against 142.83 (0.53%), each bound
    // that plus one sample of the oracle's 64 (0.016 of a pixel).
    // Screen-linear interpolation of the shape position
    // (`@interpolate(linear)`) gives a worst of 1 and 2% more area
    let worst = drawn
        .iter()
        .zip(&oracle)
        .map(|(d, o)| (d - o).abs())
        .fold(0.0, f32::max);
    let (ours, theirs) = (drawn.iter().sum::<f32>(), oracle.iter().sum::<f32>());
    assert!(
        theirs > 40.0,
        "the shapes are in view: {theirs} pixels covered"
    );
    assert!(
        worst <= 0.17,
        "a pixel differs from the CPU drawing by {worst}"
    );
    assert!(
        (ours - theirs).abs() <= theirs * 0.0075,
        "covered area {ours}, the CPU drawing's {theirs}"
    );
}

/// Ground from plane y `near` to 50 units ahead (the plane turned flat,
/// its +Y running away from the eye, 1 unit below it), each pixel's
/// coverage.
fn ground_from(near: f32) -> Vec<f32> {
    let ground = white(
        &rect([-50.0, near], [100.0, 50.0 - near], 0.0),
        None,
        scaled(1.0),
    );
    let world = Mat4::from_translation(Vec3::new(0.0, -1.0, 0.0))
        * Mat4::from_rotation_x(-90f32.to_radians());
    coverage_3d(SIDE, &[ground], perspective(), world)
}

#[test]
fn gpu_a_plane_reaching_behind_or_to_the_camera_draws_the_part_in_front() {
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

#[test]
fn gpu_a_shape_over_the_cleared_backing_leaves_its_coverage_as_alpha() {
    // Given a circle over a transparent backing, as a scene's is cleared
    let shape = white(&circle([16.0, 16.0], 9.6), None, scaled(1.0));
    let clear = vapors_scene::Color::from_rgba8(0, 0, 0, 0);

    // When
    let pixels = pixels_3d(SIDE, &[shape], (pixel_camera(SIDE), Mat4::IDENTITY), clear);

    // Then each pixel's alpha is its coverage, which its colour carries
    // too, so the painter's composite of the surface reads coverage once
    let edges = pixels.chunks(4).filter(|p| p[3] > 0 && p[3] < 255).count();
    assert!(edges > 0, "the circle has edge pixels");
    for (at, p) in pixels.chunks(4).enumerate() {
        assert!(
            p[0].abs_diff(p[3]) <= 1,
            "pixel {at}: colour {} alpha {}",
            p[0],
            p[3]
        );
    }
}
