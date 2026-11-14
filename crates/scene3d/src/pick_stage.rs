//! A scene to pick in, shared by the pick tests.

use crate::{
    CameraHandle, Colour, GeometryHandle, Meshes, NodeId, NodeKind, PickAt, PickError, Projection,
    Scene, Transform, Vec3, pick,
};

/// A scene viewed by a camera 5 back on +Z looking down -Z through a
/// projection, on a surface of `size` pixels, with a unit square's
/// geometry.
pub(crate) struct Stage {
    pub(crate) scene: Scene,
    pub(crate) meshes: Meshes,
    pub(crate) size: [f32; 2],
    pub(crate) camera: NodeId,
    square: GeometryHandle,
}

impl Stage {
    /// Orthographic, 4 units high, on 100 x 100: 25 pixels a unit, the
    /// origin at pixel (50, 50).
    pub(crate) fn new() -> Self {
        Self::viewed(ortho(4.0), [100.0, 100.0])
    }

    pub(crate) fn viewed(projection: Projection, size: [f32; 2]) -> Self {
        Self::with_eye(projection, size, at(Vec3::new(0.0, 0.0, 5.0)))
    }

    /// The camera placed by `eye` instead of 5 back.
    pub(crate) fn with_eye(projection: Projection, size: [f32; 2], eye: Transform) -> Self {
        let mut scene = Scene::new();
        let (root, handle) = (scene.root(), CameraHandle::new());
        let kind = NodeKind::Camera { handle, projection };
        let camera = scene.add(root, kind, eye).unwrap();
        scene.set_camera(Some(handle));
        let mut meshes = Meshes::new();
        let square = unit_square(&mut meshes, 1.0);
        Self {
            scene,
            meshes,
            size,
            camera,
            square,
        }
    }

    pub(crate) fn square_at(&mut self, local: Transform) -> NodeId {
        let geometry = self.square.clone();
        self.mesh_at(geometry, local)
    }

    pub(crate) fn mesh_at(&mut self, geometry: GeometryHandle, local: Transform) -> NodeId {
        let kind = NodeKind::Mesh {
            geometry,
            colour: Colour::BLACK,
        };
        let root = self.scene.root();
        self.scene.add(root, kind, local).unwrap()
    }

    pub(crate) fn try_pick(&mut self, point: [f32; 2]) -> Result<Option<NodeId>, PickError> {
        let at = PickAt {
            point,
            size: self.size,
        };
        pick(&mut self.scene, &self.meshes, at, |_| false)
    }

    pub(crate) fn pick(
        &mut self,
        point: [f32; 2],
        skip: impl Fn(NodeId) -> bool,
    ) -> Option<NodeId> {
        let at = PickAt {
            point,
            size: self.size,
        };
        pick(&mut self.scene, &self.meshes, at, skip).expect("picks")
    }
}

/// A square of `side` on the XY plane, centred on the origin.
pub(crate) fn unit_square(meshes: &mut Meshes, side: f32) -> GeometryHandle {
    let half = side / 2.0;
    let corners = [(-half, -half), (half, -half), (half, half), (-half, half)];
    let positions = corners.map(|(x, y)| Vec3::new(x, y, 0.0)).to_vec();
    meshes
        .geometry(positions, vec![0, 1, 2, 0, 2, 3], vec![1.0; 4])
        .unwrap()
}

pub(crate) fn at(position: Vec3) -> Transform {
    Transform::from_translation(position).unwrap()
}

pub(crate) fn at_z(z: f32) -> Transform {
    at(Vec3::new(0.0, 0.0, z))
}

pub(crate) fn ortho(height: f32) -> Projection {
    Projection::orthographic(height, 0.1, 100.0).unwrap()
}
