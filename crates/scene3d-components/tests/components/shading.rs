//! `<Mesh shading={..}>` (lighting LT3b): the mesh a page mounts is shaded
//! as the page says, and unlit without it.

use super::support::shading_page::{ShadingPage, ShadingPageProps};
use super::support::{Stage, square};
use vapors_scene3d::Shading;

#[test]
fn a_mesh_is_shaded_as_its_page_says_and_unlit_without_a_prop() {
    // Given
    let stage = Stage::new();
    let props = ShadingPageProps {
        scene: Some(std::rc::Rc::clone(&stage.scene)),
        surface: Some(stage.surface.clone()),
        square: Some(square(&stage.meshes)),
        shading: Shading::Glossy,
    };
    let body = stage.doc.borrow().body();

    // When
    let _page = ShadingPage::mount(&stage.doc, body, props);

    // Then
    let scene = stage.scene.borrow();
    let meshes = scene.children(scene.root()).unwrap().to_vec();
    let shading = |i: usize| scene.shading(meshes[i]).unwrap();
    assert_eq!((shading(0), shading(1)), (Shading::Glossy, Shading::Unlit));
}
