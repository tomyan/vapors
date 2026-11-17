//! `<Mesh material={..}>` (one-scene R2): the mesh a page mounts is drawn
//! with the material the page passes, and with its colour without one.

use super::support::material_page::{MaterialPage, MaterialPageProps};
use super::support::{Stage, square};
use vapors_scene3d::{Material, MaterialId, Scene};

/// The material of the page's one mesh, after mounting it with `material`.
fn mounted_with(
    material: impl FnOnce(&Stage) -> Option<MaterialId>,
) -> (Option<MaterialId>, Option<MaterialId>) {
    let stage = Stage::new();
    let given = material(&stage);
    let props = MaterialPageProps {
        scene: Some(std::rc::Rc::clone(&stage.scene)),
        surface: Some(stage.surface.clone()),
        square: Some(square(&stage.meshes)),
        material: given,
    };
    let body = stage.doc.borrow().body();
    let _page = MaterialPage::mount(&stage.doc, body, props);
    let scene: std::cell::Ref<'_, Scene> = stage.scene.borrow();
    let mesh = scene.children(scene.root()).unwrap()[0];
    (given, scene.material(mesh).unwrap())
}

#[test]
fn a_mesh_given_a_material_is_drawn_with_it() {
    let (given, held) = mounted_with(|stage| {
        let material = Material {
            label: "plain".into(),
            wgsl: "fn material(at: MaterialInput) -> vec4<f32> { return at.colour; }".into(),
        };
        Some(stage.meshes.borrow_mut().material(material).expect("held"))
    });
    assert!(given.is_some());
    assert_eq!(held, given);
}

#[test]
fn a_mesh_given_no_material_is_drawn_with_its_colour() {
    assert_eq!(mounted_with(|_| None), (None, None));
}
