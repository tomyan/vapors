use super::*;
use crate::{
    CameraHandle, Colour, GeometryHandle, Meshes, NodeKind, Projection, Quat, Transform, Vec3,
};

/// A scene viewed by a camera 5 back on +Z looking down -Z through
/// `projection`, on a 100 x 100 surface, with its geometry.
struct Stage {
    scene: Scene,
    meshes: Meshes,
    square: GeometryHandle,
}

impl Stage {
    fn new(projection: Projection) -> Self {
        let mut scene = Scene::new();
        let (root, handle) = (scene.root(), CameraHandle::new());
        let eye = Transform::from_translation(Vec3::new(0.0, 0.0, 5.0)).unwrap();
        scene
            .add(root, NodeKind::Camera { handle, projection }, eye)
            .unwrap();
        scene.set_camera(Some(handle));
        let mut meshes = Meshes::new();
        let corners = [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)];
        let positions = corners.map(|(x, y)| Vec3::new(x, y, 0.0)).to_vec();
        let square = meshes
            .geometry(positions, vec![0, 1, 2, 0, 2, 3], vec![1.0; 4])
            .unwrap();
        Self {
            scene,
            meshes,
            square,
        }
    }

    fn square_at(&mut self, local: Transform) -> NodeId {
        let kind = NodeKind::Mesh {
            geometry: self.square.clone(),
            colour: Colour::BLACK,
        };
        let root = self.scene.root();
        self.scene.add(root, kind, local).unwrap()
    }

    fn pick(&mut self, point: [f32; 2], skip: impl Fn(NodeId) -> bool) -> Option<NodeId> {
        let at = PickAt {
            point,
            size: [100.0, 100.0],
        };
        pick(&mut self.scene, &self.meshes, at, skip).expect("picks")
    }
}

fn at(z: f32) -> Transform {
    Transform::from_translation(Vec3::new(0.0, 0.0, z)).unwrap()
}

fn ortho() -> Projection {
    Projection::orthographic(4.0, 0.1, 100.0).unwrap()
}

#[test]
fn the_nearest_mesh_under_the_point_is_picked_not_one_behind_it() {
    // Given: a square at z 0 behind one at z 1, both under the centre
    let mut stage = Stage::new(ortho());
    let _behind = stage.square_at(at(0.0));
    let front = stage.square_at(at(1.0));

    // Then
    assert_eq!(stage.pick([50.0, 50.0], |_| false), Some(front));
}

#[test]
fn a_skipped_mesh_lets_the_pick_through_to_the_one_behind() {
    // Given
    let mut stage = Stage::new(ortho());
    let behind = stage.square_at(at(0.0));
    let front = stage.square_at(at(1.0));

    // Then
    assert_eq!(stage.pick([50.0, 50.0], |node| node == front), Some(behind));
}

#[test]
fn a_point_outside_every_mesh_picks_nothing() {
    // Given: 25 pixels a unit, so the square covers 37.5 to 62.5
    let mut stage = Stage::new(ortho());
    stage.square_at(at(0.0));

    // Then
    assert_eq!(stage.pick([30.0, 50.0], |_| false), None);
    assert!(
        stage.pick([40.0, 60.0], |_| false).is_some(),
        "inside, off centre"
    );
}

#[test]
fn a_hidden_mesh_is_not_picked() {
    // Given
    let mut stage = Stage::new(ortho());
    let behind = stage.square_at(at(0.0));
    let front = stage.square_at(at(1.0));
    stage
        .scene
        .set_appearance(front, Colour::BLACK, false)
        .unwrap();

    // Then
    assert_eq!(stage.pick([50.0, 50.0], |_| false), Some(behind));
}

#[test]
fn a_mesh_seen_edge_on_is_missed_and_its_transform_is_followed() {
    // Given: the square turned a quarter about +Y, and one moved right
    let mut stage = Stage::new(ortho());
    let edge_on = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    stage.square_at(Transform::new(Vec3::ZERO, edge_on, Vec3::ONE).unwrap());
    let right = stage.square_at(Transform::from_translation(Vec3::new(1.0, 0.0, 0.0)).unwrap());

    // Then: 1 unit right is pixel 75
    assert_eq!(stage.pick([45.0, 50.0], |_| false), None);
    assert_eq!(stage.pick([75.0, 50.0], |_| false), Some(right));
}

#[test]
fn a_perspective_camera_picks_along_its_rays() {
    // Given: a square 2 back at x 1, off the centre ray
    let mut stage = Stage::new(Projection::perspective(1.0, 0.1, 100.0).unwrap());
    let off = stage.square_at(Transform::from_translation(Vec3::new(1.0, 0.0, -2.0)).unwrap());
    let view_projection = stage.scene_view_projection();

    // When: where its centre projects
    let ndc = crate::project(view_projection, Vec3::new(1.0, 0.0, -2.0)).unwrap();
    let point = [(ndc.x + 1.0) * 50.0, (1.0 - ndc.y) * 50.0];

    // Then
    assert_eq!(stage.pick(point, |_| false), Some(off));
    assert_eq!(stage.pick([50.0, 50.0], |_| false), None);
}

impl Stage {
    fn scene_view_projection(&mut self) -> crate::Mat4 {
        self.scene.update();
        let camera = self.scene.active_camera().unwrap();
        let NodeKind::Camera { projection, .. } = self.scene.kind(camera).unwrap().clone() else {
            panic!("a camera")
        };
        projection.matrix(1.0).unwrap() * self.scene.world(camera).unwrap().inverse()
    }
}

#[test]
fn a_scene_without_a_camera_is_refused() {
    // Given
    let mut scene = Scene::new();
    let meshes = Meshes::new();

    // When
    let at = PickAt {
        point: [1.0, 1.0],
        size: [10.0, 10.0],
    };
    let refused = pick(&mut scene, &meshes, at, |_| false);

    // Then
    assert_eq!(refused, Err(PickError::Store(StoreError::NoCamera)));
}
