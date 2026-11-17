//! Materials for the scene's material tests (`shade/materials.rs`,
//! `refusals/materials.rs`).

use vapors_scene3d::{Material, MaterialId, NodeId, Projection, Transform, Vec3};

use super::Fixture;

pub const RED: [u8; 4] = [255, 0, 0, 255];
pub const BLUE: [u8; 4] = [0, 0, 255, 255];

/// A material whose WGSL body is `body`, given `at`.
pub fn returning(label: &str, body: &str) -> Material {
    Material {
        label: label.into(),
        wgsl: format!("fn material(at: MaterialInput) -> vec4<f32> {{ {body} }}"),
    }
}

/// A material drawing `rgba` everywhere.
pub fn flat(label: &str, [r, g, b, _]: [u8; 4]) -> Material {
    let [r, g, b] = [r, g, b].map(|c| f32::from(c) / 255.0);
    returning(label, &format!("return vec4({r:?}, {g:?}, {b:?}, 1.0);"))
}

/// A fixture looking straight down -Z through a view 4 high: 30 backing
/// pixels a unit, the origin at the backing's centre.
pub fn looking_down() -> Fixture {
    let fixture = Fixture::new();
    fixture.camera(
        Projection::orthographic(4.0, 0.1, 100.0).unwrap(),
        Vec3::new(0.0, 0.0, 5.0),
    );
    fixture
}

/// A 4-wide square at the origin, filling the view's height, in `rgba`,
/// and its node.
pub fn square_in_view(rgba: [u8; 4]) -> (Fixture, NodeId) {
    let fixture = looking_down();
    let square = fixture.square(2.0);
    let node = fixture.mesh(fixture.root(), &square, rgba, Transform::IDENTITY);
    (fixture, node)
}

/// `material` given to the fixture's `Meshes` and set on `node`.
pub fn with_material(fixture: &Fixture, node: NodeId, material: Material) -> MaterialId {
    let id = fixture
        .meshes
        .borrow_mut()
        .material(material)
        .expect("held");
    set(fixture, node, id);
    id
}

/// `node` drawn with `material`.
pub fn set(fixture: &Fixture, node: NodeId, material: MaterialId) {
    let mut scene = fixture.scene.borrow_mut();
    scene.set_material(node, Some(material)).unwrap();
}
