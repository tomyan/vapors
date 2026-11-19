//! `Shading::Glossy`'s `Blinn-Phong` highlight against the CPU's formula
//! (`docs/plans/lighting.md`, `LT3b`), seen through a perspective camera
//! (the view direction changes across the face) and through an
//! orthographic one (it does not); `Shading::Lit` has none.

mod support;

use support::lit::{box_mesh, colour_at, encoded, near, shining, sun};
use support::{Fixture, render};
use vapors_scene3d::{Ambient, Colour, Projection, Quat, Shading, Transform, Vec3};

const ALBEDO: [u8; 4] = [204, 128, 77, 255];
const AMBIENT: f32 = 0.1;
const EYE: Vec3 = Vec3::new(0.0, 0.0, 5.0);
/// Where the front face (z = 1) reflects this light to the perspective eye.
const PEAK: Vec3 = Vec3::new(0.8, 0.0, 1.0);

fn to_light() -> Vec3 {
    Vec3::new(0.2, 0.0, 1.0).normalize()
}

/// The formula at `world` on the front face, the eye along `view`.
fn expected(view: Vec3, gloss: f32) -> [f32; 3] {
    let lambert = to_light().z;
    let half = (to_light() + view).normalize();
    encoded(ALBEDO, AMBIENT + lambert, gloss * half.z.powf(32.0))
}

fn render_box(projection: Projection, shading: Shading) -> (Fixture, Vec<u8>) {
    let gpu = vapors_gpu_harness::gpu();
    let fixture = Fixture::new();
    fixture.camera(projection, EYE);
    let scale = Transform::new(Vec3::ZERO, Quat::IDENTITY, Vec3::splat(2.0)).unwrap();
    box_mesh(&fixture, ALBEDO, scale, shading);
    sun(&fixture, shining(-to_light()));
    let white = Colour::srgb(1.0, 1.0, 1.0).unwrap();
    let ambient = Ambient::new(white, AMBIENT).unwrap();
    fixture.scene.borrow_mut().set_ambient(ambient);
    let mut renderer = fixture.renderer(&gpu);
    let pixels = render(&mut renderer, &gpu).expect("renders");
    (fixture, pixels)
}

fn perspective() -> Projection {
    Projection::perspective(1.0, 0.1, 100.0).unwrap()
}

#[test]
fn gpu_a_glossy_box_under_a_perspective_camera_peaks_where_the_eye_meets_the_reflection() {
    // Given / When
    let (fixture, pixels) = render_box(perspective(), Shading::Glossy);

    // Then: the peak and a point away from it each match the formula
    for at in [PEAK, Vec3::new(-0.8, 0.0, 1.0)] {
        let got = colour_at(&pixels, &fixture, at);
        let want = expected((EYE - at).normalize(), 0.5);
        assert!(near(got, want), "at {at}: {got:?}, wanted {want:?}");
    }
    let flat = expected(Vec3::Z, 0.5);
    let got = colour_at(&pixels, &fixture, PEAK);
    assert!(
        !near(got, flat),
        "the peak must not be the orthographic one"
    );
}

#[test]
fn gpu_a_glossy_box_under_an_orthographic_camera_views_along_the_camera_axis() {
    // Given / When
    let ortho = Projection::orthographic(6.0, 0.1, 100.0).unwrap();
    let (fixture, pixels) = render_box(ortho, Shading::Glossy);

    // Then: every point has the view +Z
    for at in [PEAK, Vec3::new(0.0, 0.0, 1.0)] {
        let got = colour_at(&pixels, &fixture, at);
        let want = expected(Vec3::Z, 0.5);
        assert!(near(got, want), "at {at}: {got:?}, wanted {want:?}");
    }
}

#[test]
fn gpu_a_lit_box_has_no_highlight() {
    // Given / When
    let (fixture, pixels) = render_box(perspective(), Shading::Lit);

    // Then: the peak is the diffuse colour alone
    let got = colour_at(&pixels, &fixture, PEAK);
    let want = expected((EYE - PEAK).normalize(), 0.0);
    assert!(near(got, want), "{got:?}, wanted {want:?}");
}
