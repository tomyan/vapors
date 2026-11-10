//! The ground and object check (docs/plans/3d-2.md): a marker the
//! ground carries at world point P and an object standing at P land on
//! the pixel `project` gives for P, through one camera moved after
//! mount, under both projections.

mod support;

use support::{Fixture, GREEN, RED, SIZE, centroid, render, to_pixel};
use vapors_gpu_harness::Gpu;
use vapors_scene3d::{
    GeometryId, NodeId, Projection, Transform, Vec3, View, project, view_projection,
};
use vapors_scene3d_gpu::SceneRenderer;

const BLUE: [u8; 4] = [0, 0, 255, 255];
/// Where the marker is and the object stands.
const P: Vec3 = Vec3::new(0.6, 0.0, -0.4);
/// Above the ground by a hair, so the marker is not drawn at the
/// ground's depth; under a hundredth of a pixel here.
const LIFT: f32 = 0.001;

/// A quad through `corners`, two triangles.
fn quad(fixture: &Fixture, corners: [Vec3; 4]) -> GeometryId {
    let indices = vec![0, 1, 2, 0, 2, 3];
    let mut meshes = fixture.meshes.borrow_mut();
    meshes.geometry(corners.to_vec(), indices).unwrap()
}

/// A square `2 * half` wide in the XZ plane, centred on the origin.
fn flat(fixture: &Fixture, half: f32) -> GeometryId {
    let corners = [(-half, -half), (half, -half), (half, half), (-half, half)];
    quad(fixture, corners.map(|(x, z)| Vec3::new(x, 0.0, z)))
}

/// A post 0.1 wide and 1 high in the XY plane, its base centred on the origin.
fn post(fixture: &Fixture) -> GeometryId {
    let corners = [(-0.05, 0.0), (0.05, 0.0), (0.05, 1.0), (-0.05, 1.0)];
    quad(fixture, corners.map(|(x, y)| Vec3::new(x, y, 0.0)))
}

/// The camera transform of a view from `eye` towards the origin.
fn looking_from(eye: Vec3) -> (Transform, View) {
    let view = View::look_at(eye, Vec3::ZERO, Vec3::Y).unwrap();
    let (_, rotation, translation) = view.matrix().inverse().to_scale_rotation_translation();
    (
        Transform::new(translation, rotation, Vec3::ONE).unwrap(),
        view,
    )
}

fn at(point: Vec3) -> Transform {
    Transform::from_translation(point).unwrap()
}

/// The centre of the bottom edge of the pixels exactly `rgba`.
#[allow(clippy::cast_precision_loss)]
fn base(pixels: &[u8], rgba: [u8; 4]) -> [f32; 2] {
    let width = SIZE.width as usize;
    let rows = pixels.as_chunks::<4>().0.chunks(width);
    let (y, row) = rows
        .enumerate()
        .rev()
        .find(|(_, row)| row.contains(&rgba))
        .expect("the object is drawn");
    let xs: Vec<usize> = (0..width).filter(|x| row[*x] == rgba).collect();
    let x = xs.iter().sum::<usize>() as f32 / xs.len() as f32 + 0.5;
    [x, y as f32 + 1.0]
}

/// The ground carrying its marker at P, drawn once from one place;
/// the fixture, its renderer and the camera's node.
fn ground_drawn(gpu: &Gpu, projection: Projection) -> (Fixture, SceneRenderer, NodeId) {
    let fixture = Fixture::new();
    let camera = fixture.camera(projection, Vec3::new(0.0, 3.0, 6.0));
    let ground = fixture.mesh(fixture.root(), flat(&fixture, 4.0), GREEN, at(Vec3::ZERO));
    let square = flat(&fixture, 0.3);
    fixture.mesh(ground, square, RED, at(P + Vec3::new(0.0, LIFT, 0.0)));
    let mut renderer = fixture.renderer(gpu);
    render(&mut renderer, gpu).expect("renders");
    (fixture, renderer, camera)
}

/// Where `project` puts P through `projection` and `view`.
#[allow(clippy::cast_precision_loss)]
fn projected(projection: &Projection, view: &View) -> [f32; 2] {
    let aspect = SIZE.width as f32 / SIZE.height as f32;
    let matrix = view_projection(projection, view, aspect).unwrap();
    to_pixel(project(matrix, P).expect("in front of the eye"))
}

/// With the camera moved after the ground was drawn: the marker's
/// centre, then the object's base with the object added, and where
/// `project` puts P for the moved camera.
fn landed(projection: Projection) -> ([f32; 2], [f32; 2], [f32; 2]) {
    let gpu = vapors_gpu_harness::gpu();
    let (fixture, mut renderer, camera) = ground_drawn(&gpu, projection);
    let (moved, view) = looking_from(Vec3::new(-1.5, 2.5, 5.0));
    let mut scene = fixture.scene.borrow_mut();
    scene.set_transform(camera, moved).unwrap();
    drop(scene);
    let (marker, count) = centroid(&render(&mut renderer, &gpu).expect("renders"), RED);
    assert!(count > 20, "the marker covers {count} pixels");
    fixture.mesh(fixture.root(), post(&fixture), BLUE, at(P));
    let object = base(&render(&mut renderer, &gpu).expect("renders"), BLUE);
    (marker, object, projected(&projection, &view))
}

fn assert_within_a_pixel(what: &str, landed: [f32; 2], expected: [f32; 2]) {
    let distance = ((landed[0] - expected[0]).powi(2) + (landed[1] - expected[1]).powi(2)).sqrt();
    assert!(
        distance <= 1.0,
        "{what} landed at {landed:?}, project gives {expected:?}"
    );
}

#[test]
fn gpu_the_marker_and_the_object_land_where_project_puts_p_in_perspective() {
    let (marker, object, expected) = landed(Projection::perspective(1.0, 0.1, 100.0).unwrap());
    assert_within_a_pixel("the marker", marker, expected);
    assert_within_a_pixel("the object's base", object, expected);
}

#[test]
fn gpu_the_marker_and_the_object_land_where_project_puts_p_in_orthographic() {
    let (marker, object, expected) = landed(Projection::orthographic(6.0, 0.1, 100.0).unwrap());
    assert_within_a_pixel("the marker", marker, expected);
    assert_within_a_pixel("the object's base", object, expected);
}
